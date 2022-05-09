use crate::{clients::Cli, core::Docker};
use conquer_once::Lazy;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use std::{collections::BTreeSet, sync::Mutex, thread};

static WATCHDOG: Lazy<Mutex<Watchdog>> = Lazy::new(|| {
    thread::spawn(move || {
        let signal_docker = Cli::default();
        let mut signals =
            Signals::new(&[SIGTERM, SIGINT, SIGQUIT]).expect("failed to register signal handler");

        for signal in &mut signals {
            for container_id in WATCHDOG
                .lock()
                .map(|s| s.containers.clone())
                .unwrap_or_default()
            {
                signal_docker.stop(&container_id);
                signal_docker.rm(&container_id);
            }

            let _ = signal_hook::low_level::emulate_default_handler(signal);
        }
    });

    Mutex::new(Watchdog::default())
});

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
