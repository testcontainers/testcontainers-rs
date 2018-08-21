use api::*;
use serde_json;
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

#[derive(Copy, Clone)]
pub struct DockerCli;

impl Docker for DockerCli {
    fn new() -> Self {
        DockerCli
    }

    fn run<I: Image>(&self, image: I) -> Container<DockerCli, I> {
        let mut docker = Command::new("docker");

        let command = docker
            .arg("run")
            .arg("-d") // Always run detached
            .arg("-P") // Always expose all ports
            .arg(&image.descriptor())
            .args(image.args())
            .stdout(Stdio::piped());

        info!("Executing command: {:?}", command);

        let child = command.spawn().expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();
        let reader = BufReader::new(stdout);

        let container_id = reader.lines().next().unwrap().unwrap();

        // TODO maybe move log statements to container
        let container = Container::new(container_id, DockerCli {}, image);

        debug!("Waiting for {} to be ready.", container);

        container.block_until_ready();

        debug!("{} is now ready!", container);

        container
    }

    fn logs(&self, id: &str) -> Box<Read> {
        // Hack to fix unstable CI builds. Sometimes the logs are not immediately available after starting the container.
        // Let's sleep for a little bit of time to let the container start up before we actually process the logs.
        sleep(Duration::from_millis(100));

        let child = Command::new("docker")
            .arg("logs")
            .arg("-f")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        Box::new(child.stdout.unwrap())
    }

    fn inspect(&self, id: &str) -> ContainerInfo {
        let child = Command::new("docker")
            .arg("inspect")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();

        let mut infos: Vec<ContainerInfo> = serde_json::from_reader(stdout).unwrap();

        let info = infos.remove(0);

        trace!("Fetched container info: {:#?}", info);

        info
    }

    fn rm(&self, id: &str) {
        info!("Killing docker container: {}", id);

        Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg("-v") // Also remove volumes
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }

    fn stop(&self, id: &str) {
        info!("Stopping docker container: {}", id);

        Command::new("docker")
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }
}
