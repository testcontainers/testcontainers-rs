//! Watchdog that stops and removes containers on SIGTERM, SIGINT or SIGQUIT, and force-removes them on a second signal.
//!
//! By default, the watchdog is disabled. To enable it, enable the `watchdog` feature.
//! Note that it works in background thread and may panic.

use std::{collections::BTreeSet, sync::Mutex, thread};

use conquer_once::Lazy;
use futures::future::join_all;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use tokio::signal::unix::{signal, Signal, SignalKind};

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
            let mut signals =
                TerminationSignals::register().expect("failed to register signal handler");

            let signal = signals.recv().await;
            let containers = registered_containers();
            tokio::select! {
                () = stop_and_remove(&docker, &containers) => {}
                _ = signals.recv() => force_remove(&docker, &containers).await,
            }

            let _ = signal_hook::low_level::emulate_default_handler(signal);
        });
    });

    Mutex::new(Watchdog::default())
});

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
