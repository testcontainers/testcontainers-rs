#![cfg(all(unix, feature = "watchdog"))]

use std::{
    collections::HashMap,
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bollard::query_parameters::{ListContainersOptionsBuilder, RemoveContainerOptionsBuilder};
use testcontainers::{
    core::{client::docker_client_instance, Mount, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

const LABEL_KEY: &str = "org.testcontainers.watchdog-test";
const LABEL_ENV: &str = "WATCHDOG_TEST_LABEL";
const COUNT_ENV: &str = "WATCHDOG_TEST_COUNT";
const MOUNT_ENV: &str = "WATCHDOG_TEST_MOUNT";
const SIGINT: i32 = 2;
const STARTUP_LIMIT: Duration = Duration::from_secs(120);
const EXIT_LIMIT: Duration = Duration::from_secs(20);

/// Holds the requested containers until a signal ends the process, when re-executed by the tests below.
#[tokio::test]
#[ignore = "re-executed by the other tests in this file"]
async fn watchdog_child() {
    let label = std::env::var(LABEL_ENV).expect("label");
    let count: usize = std::env::var(COUNT_ENV)
        .expect("count")
        .parse()
        .expect("count is a number");
    let mount = std::env::var(MOUNT_ENV).ok();

    let mut containers = Vec::with_capacity(count);
    for _ in 0..count {
        let request = match &mount {
            // PID 1 has no TERM handler, so `docker stop` waits for its timeout.
            None => GenericImage::new("alpine", "3").with_cmd(["sleep", "3600"]),
            Some(dir) => GenericImage::new("alpine", "3")
                .with_wait_for(WaitFor::message_on_stdout("ready"))
                .with_cmd([
                    "sh",
                    "-c",
                    "trap 'echo graceful > /out/stopped; exit 0' TERM; echo ready; while :; do sleep 0.1; done",
                ])
                .with_mount(Mount::bind_mount(dir.as_str(), "/out")),
        };
        containers.push(
            request
                .with_label(LABEL_KEY, label.as_str())
                .start()
                .await
                .expect("container starts"),
        );
    }

    println!("READY {label}");
    std::future::pending::<()>().await;
}

struct Interrupted {
    status: ExitStatus,
    elapsed: Duration,
    leftover: Vec<String>,
}

async fn spawn_child(label: &str, count: usize, mount: Option<&Path>) -> Child {
    let mut command = Command::new(std::env::current_exe().expect("test binary path"));
    command
        .args(["watchdog_child", "--exact", "--ignored", "--nocapture"])
        .env(LABEL_ENV, label)
        .env(COUNT_ENV, count.to_string())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    if let Some(dir) = mount {
        command.env(MOUNT_ENV, dir);
    }
    let mut child = command.spawn().expect("child starts");

    let mut lines = BufReader::new(child.stdout.take().expect("piped stdout")).lines();
    let ready = format!("READY {label}");
    tokio::time::timeout(STARTUP_LIMIT, async {
        while let Some(line) = lines.next_line().await.expect("child stdout") {
            if line == ready {
                return;
            }
        }
        panic!("child exited before starting its containers");
    })
    .await
    .expect("child starts its containers in time");

    // The watchdog registers its signal handlers on a background thread after the first container.
    tokio::time::sleep(Duration::from_millis(500)).await;
    child
}

fn interrupt(child: &Child) {
    let pid = child.id().expect("child is running").to_string();
    let status = std::process::Command::new("kill")
        .args(["-INT", &pid])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -INT {pid} failed");
}

async fn wait_and_collect(mut child: Child, label: &str, started: Instant) -> Interrupted {
    let status = match tokio::time::timeout(EXIT_LIMIT, child.wait()).await {
        Ok(status) => status.expect("child wait"),
        Err(_) => {
            child.kill().await.expect("kill child");
            remove_labelled(label).await;
            panic!("child still running {EXIT_LIMIT:?} after the first interrupt");
        }
    };
    let elapsed = started.elapsed();
    let leftover = remove_labelled(label).await;
    Interrupted {
        status,
        elapsed,
        leftover,
    }
}

/// Force-removes the containers carrying `label` and returns their ids.
async fn remove_labelled(label: &str) -> Vec<String> {
    let docker = docker_client_instance().await.expect("docker client");
    let filters = HashMap::from([("label".to_string(), vec![format!("{LABEL_KEY}={label}")])]);
    let ids: Vec<String> = docker
        .list_containers(Some(
            ListContainersOptionsBuilder::new()
                .all(true)
                .filters(&filters)
                .build(),
        ))
        .await
        .expect("list containers")
        .into_iter()
        .filter_map(|summary| summary.id)
        .collect();
    for id in &ids {
        let _ = docker
            .remove_container(
                id,
                Some(
                    RemoveContainerOptionsBuilder::new()
                        .force(true)
                        .v(true)
                        .build(),
                ),
            )
            .await;
    }
    ids
}

fn unique_label(test: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    format!("{test}-{}-{nanos}", std::process::id())
}

#[tokio::test]
async fn interrupt_removes_every_container_within_grace() {
    let label = unique_label("grace");
    let child = spawn_child(&label, 3, None).await;

    let started = Instant::now();
    interrupt(&child);
    let outcome = wait_and_collect(child, &label, started).await;

    assert_eq!(outcome.status.signal(), Some(SIGINT));
    assert!(
        outcome.elapsed < Duration::from_secs(8),
        "cleanup took {:?}",
        outcome.elapsed
    );
    assert_eq!(outcome.leftover, Vec::<String>::new());
}

#[tokio::test]
async fn second_interrupt_forces_removal() {
    let label = unique_label("second");
    let child = spawn_child(&label, 1, None).await;

    let started = Instant::now();
    interrupt(&child);
    tokio::time::sleep(Duration::from_secs(1)).await;
    interrupt(&child);
    let outcome = wait_and_collect(child, &label, started).await;

    assert_eq!(outcome.status.signal(), Some(SIGINT));
    assert!(
        outcome.elapsed < Duration::from_secs(3),
        "cleanup took {:?}",
        outcome.elapsed
    );
    assert_eq!(outcome.leftover, Vec::<String>::new());
}

#[tokio::test]
async fn interrupt_stops_containers_gracefully() {
    let label = unique_label("graceful");
    let dir = tempfile::tempdir().expect("temp dir");
    let child = spawn_child(&label, 1, Some(dir.path())).await;

    let started = Instant::now();
    interrupt(&child);
    let outcome = wait_and_collect(child, &label, started).await;

    assert_eq!(outcome.status.signal(), Some(SIGINT));
    assert_eq!(outcome.leftover, Vec::<String>::new());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("stopped")).expect("TERM trap ran"),
        "graceful\n"
    );
}
