//! Watchdog that stops and removes containers on SIGTERM, SIGINT or SIGQUIT, and force-removes them on a second signal.
//!
//! By default, the watchdog is disabled. To enable it, enable the `watchdog` feature.
//! Its signal handlers are installed when the first container is registered, and the cleanup runs on a background thread.

use std::{
    collections::BTreeSet,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use conquer_once::OnceCell;
use futures::future::join_all;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use tokio::{
    runtime::Runtime,
    signal::unix::{signal, Signal, SignalKind},
};

use crate::core::client::Client;

/// Seconds Docker gives a container to stop before killing it, small enough to fit nextest's 10 second grace period.
const STOP_TIMEOUT_SECS: i32 = 5;
/// How long removal waits for the stop request, since Docker answers it only once a killed container has exited.
const STOP_REQUEST_LIMIT: Duration = Duration::from_secs(7);

static WATCHDOG: OnceCell<Mutex<Watchdog>> = OnceCell::uninit();

/// Installs the signal handlers before returning, so a signal from then on reaches the background thread.
fn arm(docker: Arc<Client>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            log::error!("Failed to start the watchdog runtime, signals keep their default behaviour: {error}");
            return;
        }
    };
    let (sender, receiver) = mpsc::sync_channel(1);
    // The thread starts before any handler is installed, so a failed spawn leaves no signal unhandled.
    let spawned = thread::Builder::new()
        .name("testcontainers-watchdog".into())
        .spawn(move || watch(&docker, &receiver));
    if let Err(error) = spawned {
        log::error!(
            "Failed to start the watchdog thread, signals keep their default behaviour: {error}"
        );
        // Dropping a runtime inside the caller's async context panics.
        runtime.shutdown_background();
        return;
    }
    let signals = {
        let _context = runtime.enter();
        TerminationSignals::register()
    };
    if let Err(mpsc::SendError((runtime, _))) = sender.send((runtime, signals)) {
        runtime.shutdown_background();
    }
}

fn watch(docker: &Client, receiver: &mpsc::Receiver<(Runtime, TerminationSignals)>) {
    let Ok((runtime, mut signals)) = receiver.recv() else {
        return;
    };
    runtime.block_on(async {
        let signal = signals.recv().await;
        let containers = registered_containers();
        tokio::select! {
            () = stop_and_remove(docker, &containers) => {}
            _ = signals.recv() => force_remove(docker, &containers).await,
        }

        let _ = signal_hook::low_level::emulate_default_handler(signal);
    });
}

/// A signal whose handler failed to install is `None` and keeps its default behaviour.
struct TerminationSignals {
    interrupt: Option<Signal>,
    terminate: Option<Signal>,
    quit: Option<Signal>,
}

impl TerminationSignals {
    /// Must run inside the watchdog runtime's context.
    fn register() -> Self {
        Self {
            interrupt: install(SignalKind::interrupt(), "SIGINT"),
            terminate: install(SignalKind::terminate(), "SIGTERM"),
            quit: install(SignalKind::quit(), "SIGQUIT"),
        }
    }

    /// Waits for the next termination signal and returns its number.
    async fn recv(&mut self) -> i32 {
        tokio::select! {
            () = next(&mut self.interrupt) => SIGINT,
            () = next(&mut self.terminate) => SIGTERM,
            () = next(&mut self.quit) => SIGQUIT,
        }
    }
}

fn install(kind: SignalKind, name: &str) -> Option<Signal> {
    signal(kind)
        .inspect_err(|error| {
            log::error!("Failed to watch {name}, it keeps its default behaviour: {error}");
        })
        .ok()
}

async fn next(signal: &mut Option<Signal>) {
    if let Some(signal) = signal {
        if signal.recv().await.is_some() {
            return;
        }
    }
    // A closed stream never yields again, like a signal that is not watched.
    std::future::pending().await
}

fn registered_containers() -> Vec<String> {
    WATCHDOG
        .get()
        .and_then(|watchdog| watchdog.lock().ok())
        .map(|s| s.containers.iter().cloned().collect())
        .unwrap_or_default()
}

async fn stop_and_remove(docker: &Client, containers: &[String]) {
    join_all(containers.iter().map(|id| async move {
        let stop = docker.stop(id, Some(STOP_TIMEOUT_SECS));
        match tokio::time::timeout(STOP_REQUEST_LIMIT, stop).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => log::error!("Failed to stop container {id} on interrupt: {error}"),
            Err(_) => log::error!(
                "Container {id} did not stop within {STOP_REQUEST_LIMIT:?} of the interrupt"
            ),
        }
        remove(docker, id).await;
    }))
    .await;
}

async fn force_remove(docker: &Client, containers: &[String]) {
    join_all(containers.iter().map(|id| remove(docker, id))).await;
}

async fn remove(docker: &Client, id: &str) {
    if let Err(error) = docker.rm(id).await {
        log::error!("Failed to remove container {id} on interrupt: {error}");
    }
}

#[derive(Default)]
pub(crate) struct Watchdog {
    containers: BTreeSet<String>,
}

/// Register a container for observation, arming the watchdog with `docker` on the first call
pub(crate) fn register(docker: &Arc<Client>, container_id: String) {
    WATCHDOG
        .get_or_init(|| {
            arm(Arc::clone(docker));
            Mutex::default()
        })
        .lock()
        .expect("failed to access watchdog")
        .containers
        .insert(container_id);
}
/// Unregisters a container for observation
pub(crate) fn unregister(container_id: &str) {
    if let Some(watchdog) = WATCHDOG.get() {
        watchdog
            .lock()
            .expect("failed to access watchdog")
            .containers
            .remove(container_id);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::OnceLock, time::Duration};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::mpsc,
    };

    use super::stop_and_remove;
    use crate::core::{client::Client, env};

    /// nextest kills a test this long after forwarding the interrupt.
    const NEXTEST_GRACE: Duration = Duration::from_secs(10);

    static DAEMON: OnceLock<String> = OnceLock::new();

    struct FakeDaemonEnv;

    impl env::GetEnvValue for FakeDaemonEnv {
        fn get_env_value(key: &str) -> Option<String> {
            match key {
                "DOCKER_HOST" => DAEMON.get().cloned(),
                _ => None,
            }
        }
    }

    /// Never answers a stop, like Docker waiting on a container that outlives SIGKILL, and answers every removal.
    async fn serve(listener: TcpListener, removals: mpsc::UnboundedSender<String>) {
        loop {
            let (stream, _) = listener.accept().await.expect("fake daemon accepts");
            tokio::spawn(answer(stream, removals.clone()));
        }
    }

    async fn answer(mut stream: TcpStream, removals: mpsc::UnboundedSender<String>) {
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            if stream.read(&mut byte).await.expect("fake daemon reads") == 0 {
                return;
            }
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).expect("request head is UTF-8");
        if let Some(target) = head.strip_prefix("DELETE ") {
            let target = target.split(' ').next().expect("request target");
            removals
                .send(target.to_owned())
                .expect("test awaits removals");
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\n\r\n")
                .await
                .expect("fake daemon answers");
        }
        std::future::pending::<()>().await;
    }

    #[tokio::test]
    async fn removal_goes_ahead_when_the_stop_request_stalls() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake daemon binds");
        let address = listener.local_addr().expect("fake daemon address");
        DAEMON
            .set(format!("tcp://{address}"))
            .expect("only this test sets the daemon");
        let (sender, mut removals) = mpsc::unbounded_channel();
        tokio::spawn(serve(listener, sender));

        let config = env::Config::load::<FakeDaemonEnv>()
            .await
            .expect("config loads");
        let docker = Client::new_with_config(config).expect("client connects");
        tokio::spawn(async move { stop_and_remove(&docker, &["stalled".to_owned()]).await });

        let target = tokio::time::timeout(NEXTEST_GRACE, removals.recv())
            .await
            .expect("removal is requested within nextest's grace period")
            .expect("fake daemon is running");
        assert!(target.contains("/containers/stalled?"), "{target}");
    }
}
