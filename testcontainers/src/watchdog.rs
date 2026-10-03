//! Watchdog that stops and removes containers when the process is interrupted, and force-removes them on a second interrupt or a Windows console close.
//!
//! By default, the watchdog is disabled. To enable it, enable the `watchdog` feature.
//! Note that it works in background thread and may panic.

use std::{collections::BTreeSet, future::Future, sync::Mutex, thread};

use conquer_once::Lazy;
use futures::future::join_all;

use crate::core::client::Client;

/// Seconds Docker gives a container to stop before killing it, small enough to fit nextest's 10 second grace period.
const STOP_TIMEOUT_SECS: i32 = 5;

static WATCHDOG: Lazy<Mutex<Watchdog>> = Lazy::new(|| {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to start watchdog runtime in background");

        runtime.block_on(async {
            let docker = Client::lazy_client()
                .await
                .expect("failed to create docker client");
            platform::watch(&docker).await;
        });
    });

    Mutex::new(Watchdog::default())
});

#[cfg(unix)]
mod platform {
    use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
    use tokio::signal::unix::{signal, Signal, SignalKind};

    use crate::core::client::Client;

    pub(super) async fn watch(docker: &Client) {
        let mut signals =
            TerminationSignals::register().expect("failed to register signal handler");

        let signal = signals.recv().await;
        super::clean_up(docker, signals.recv()).await;

        let _ = signal_hook::low_level::emulate_default_handler(signal);
    }

    struct TerminationSignals {
        interrupt: Signal,
        terminate: Signal,
        quit: Signal,
    }

    impl TerminationSignals {
        fn register() -> std::io::Result<Self> {
            Ok(Self {
                interrupt: signal(SignalKind::interrupt())?,
                terminate: signal(SignalKind::terminate())?,
                quit: signal(SignalKind::quit())?,
            })
        }

        /// Waits for the next termination signal and returns its number.
        async fn recv(&mut self) -> i32 {
            tokio::select! {
                _ = self.interrupt.recv() => SIGINT,
                _ = self.terminate.recv() => SIGTERM,
                _ = self.quit.recv() => SIGQUIT,
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close, CtrlBreak, CtrlC, CtrlClose};

    use crate::core::client::Client;

    /// Exit code the default console control handler ends the process with.
    const STATUS_CONTROL_C_EXIT: i32 = 0xC000_013A_u32.cast_signed();

    /// Exits the way the default console handler would have, since listening for events replaced it.
    pub(super) async fn watch(docker: &Client) {
        let mut events =
            ConsoleEvents::register().expect("failed to register console control handler");

        match events.recv().await {
            ConsoleEvent::Interrupt => super::clean_up(docker, events.recv()).await,
            ConsoleEvent::Close => {
                super::force_remove(docker, &super::registered_containers()).await;
            }
        }

        std::process::exit(STATUS_CONTROL_C_EXIT);
    }

    enum ConsoleEvent {
        /// CTRL+C or CTRL+BREAK, which leave the process as long as it needs.
        Interrupt,
        /// Console close, after which Windows ends the process within 5 seconds.
        Close,
    }

    struct ConsoleEvents {
        ctrl_c: CtrlC,
        ctrl_break: CtrlBreak,
        ctrl_close: CtrlClose,
    }

    impl ConsoleEvents {
        fn register() -> std::io::Result<Self> {
            Ok(Self {
                ctrl_c: ctrl_c()?,
                ctrl_break: ctrl_break()?,
                ctrl_close: ctrl_close()?,
            })
        }

        async fn recv(&mut self) -> ConsoleEvent {
            tokio::select! {
                _ = self.ctrl_c.recv() => ConsoleEvent::Interrupt,
                _ = self.ctrl_break.recv() => ConsoleEvent::Interrupt,
                _ = self.ctrl_close.recv() => ConsoleEvent::Close,
            }
        }
    }
}

/// Stops and removes the registered containers, force-removing them if `escalation` completes first.
async fn clean_up(docker: &Client, escalation: impl Future) {
    let containers = registered_containers();
    tokio::select! {
        () = stop_and_remove(docker, &containers) => {}
        _ = escalation => force_remove(docker, &containers).await,
    }
}

fn registered_containers() -> Vec<String> {
    WATCHDOG
        .lock()
        .map(|s| s.containers.iter().cloned().collect())
        .unwrap_or_default()
}

async fn stop_and_remove(docker: &Client, containers: &[String]) {
    join_all(containers.iter().map(|id| async move {
        if let Err(error) = docker.stop(id, Some(STOP_TIMEOUT_SECS)).await {
            log::error!("Failed to stop container {id} on interrupt: {error}");
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

/// Register a container for observation
pub(crate) fn register(container_id: String) {
    WATCHDOG
        .lock()
        .expect("failed to access watchdog")
        .containers
        .insert(container_id);
}
/// Unregisters a container for observation
pub(crate) fn unregister(container_id: &str) {
    WATCHDOG
        .lock()
        .expect("failed to access watchdog")
        .containers
        .remove(container_id);
}
