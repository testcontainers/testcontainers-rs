use crate::core::{Container, Docker, Image, Logs, Ports, RunArgs};
use futures::StreamExt;
use shiplift::Docker as shiplift_docker;
use shiplift::{
    tty::TtyChunk, ContainerOptions, LogsOptions, NetworkCreateOptions, RmContainerOptions,
};
use std::fmt;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub struct Shiplift {
    client: shiplift_docker,
}

impl fmt::Debug for Shiplift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shiplift").finish()
    }
}

impl Shiplift {
    pub fn new() -> Self {
        return Shiplift {
            client: shiplift_docker::new(),
        };
    }

    pub fn get_rt(&self) -> Runtime {
        return Runtime::new().unwrap();
    }
}

impl Docker for Shiplift {
    fn run<I: Image>(&self, image: I) -> Container<'_, Shiplift, I> {
        let empty_args = RunArgs::default();
        self.run_with_args(image, empty_args)
    }

    fn run_with_args<I: Image>(&self, image: I, run_args: RunArgs) -> Container<'_, Shiplift, I> {
        // Create network
        if let Some(network) = run_args.network() {
            self.get_rt()
                .block_on(
                    self.client
                        .networks()
                        .create(&NetworkCreateOptions::builder(network.as_str()).build()),
                )
                .unwrap();
        }

        let mut options_builder = ContainerOptions::builder(image.descriptor().as_str());

        // name of the container
        if let Some(name) = run_args.name() {
            options_builder.name(name.as_str());
        }

        // environment variables
        let envs: Vec<String> = image
            .env_vars()
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // the fact .env and .volumes takes Vec<&str> instead of AsRef<str> is making
        // this more difficult than it needs to be
        let envs_str: Vec<&str> = envs.iter().map(|s| s.as_ref()).collect();
        options_builder.env(envs_str);

        // volumes
        let vols: Vec<String> = image
            .volumes()
            .into_iter()
            .map(|(orig, dest)| format!("{}:{}", orig, dest))
            .collect();
        let vols_str: Vec<&str> = vols.iter().map(|s| s.as_ref()).collect();
        options_builder.volumes(vols_str);

        // entrypoint
        if let Some(entrypoint) = image.entrypoint() {
            options_builder.entrypoint(entrypoint.as_str());
        }

        // ports
        if let Some(ports) = image.ports() {
            // TODO support UDP?
            for port in &ports {
                // casting u16 to u32
                options_builder.expose(port.internal as u32, "tcp", port.local as u32);
            }
        } else {
            options_builder.publish_all_ports();
        }

        // create the container with options
        let create_result = self
            .get_rt()
            .block_on(self.client.containers().create(&options_builder.build()))
            .unwrap();
        let container = self.client.containers().get(&create_result.id);

        // start the container
        self.get_rt().block_on(container.start()).unwrap();
        Container::new(create_result.id, self, image)
    }

    fn logs(&self, id: &str) -> Logs {
        // since we are doing spwan to fake a async method
        let id_static: &'static str = Box::leak(Box::new(String::from(id)));

        let stdout_buf = std::io::Cursor::new(Vec::new());
        let stderr_buf = std::io::Cursor::new(Vec::new());

        let stdout_arc = Arc::new(Mutex::new(stdout_buf));
        let stderr_arc = Arc::new(Mutex::new(stderr_buf));

        // use spwan to run it in the background
        self.get_rt().spawn(async move {
            let client = shiplift_docker::new();
            let mut logs_stream = client
                .containers()
                .get(id_static)
                .logs(&LogsOptions::builder().stdout(true).stderr(true).build());

            // have a back ground threa durnning and pipe bytes into a Read buffer
            while let Some(log_result) = logs_stream.next().await {
                match log_result {
                    Ok(chunk) => match chunk {
                        TtyChunk::StdOut(bytes) => {
                            stdout_arc.lock().unwrap().write_all(&bytes).unwrap();
                        }
                        TtyChunk::StdErr(bytes) => {
                            stderr_arc.lock().unwrap().write_all(&bytes).unwrap();
                        }
                        TtyChunk::StdIn(_) => unreachable!(),
                    },
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        });

        Logs {
            stdout: Box::new("placeholder".as_bytes()),
            stderr: Box::new("placeholder".as_bytes()),
        }
    }

    fn ports(&self, id: &str) -> Ports {
        let mut ports = Ports::default();
        let container_detatils = self
            .get_rt()
            .block_on(self.client.containers().get(id).inspect())
            .unwrap();

        //TODO should implement into_ports on external API port
        if let Some(inspect_ports) = container_detatils.network_settings.ports {
            for (internal, external) in inspect_ports {
                // PortMapping here is actualy a HashMap
                // NetworkSettings -> Port -> Vec<HashMap<String, String>>
                // therefore pop -> first key using next even though it's a map
                let external = match external
                    .and_then(|mut m| m.pop())
                    //XXX this is bad, need advice...
                    .map(|m| m.values().next().unwrap().clone())
                {
                    Some(port) => port,
                    None => {
                        log::debug!("Port {} is not mapped to host machine, skipping.", internal);
                        continue;
                    }
                };

                let port = internal.split('/').next().unwrap();

                let internal = parse_port(port);
                let external = parse_port(&external);

                ports.add_mapping(internal, external);
            }
        }

        ports
    }

    fn rm(&self, id: &str) {
        self.get_rt()
            .block_on(
                self.client.containers().get(id).remove(
                    RmContainerOptions::builder()
                        .volumes(true)
                        .force(true)
                        .build(),
                ),
            )
            .unwrap();
    }

    fn stop(&self, id: &str) {
        self.get_rt()
            .block_on(self.client.containers().get(id).stop(Option::None))
            .unwrap();
    }

    fn start(&self, id: &str) {
        self.get_rt()
            .block_on(self.client.containers().get(id).stop(Option::None))
            .unwrap();
    }
}

fn parse_port(port: &str) -> u16 {
    port.parse()
        .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
}

#[cfg(test)]
mod tests {
    use crate::core::Port;
    use crate::{Container, Docker, Image};
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct HelloWorld {
        volumes: HashMap<String, String>,
        env_vars: HashMap<String, String>,
    }

    impl Image for HelloWorld {
        type Args = Vec<String>;
        type EnvVars = HashMap<String, String>;
        type Volumes = HashMap<String, String>;
        type EntryPoint = std::convert::Infallible;

        fn descriptor(&self) -> String {
            String::from("hello-world")
        }

        fn wait_until_ready<D: Docker>(&self, _container: &Container<'_, D, Self>) {}

        fn args(&self) -> <Self as Image>::Args {
            vec![]
        }

        fn volumes(&self) -> Self::Volumes {
            self.volumes.clone()
        }

        fn env_vars(&self) -> Self::EnvVars {
            self.env_vars.clone()
        }

        fn ports(&self) -> Option<Vec<Port>> {
            None
        }

        fn with_args(self, _arguments: <Self as Image>::Args) -> Self {
            self
        }
    }

    #[test]
    fn shiplift_can_run_container() {
        let image = HelloWorld::default();
        let shiplift = Shiplift::new();
        shiplift.run(image);
    }

    #[test]
    fn shiplift_run_command_should_include_name() {
        let image = HelloWorld::default();
        let run_args = RunArgs::default().with_name("hello_container");

        let shiplift = Shiplift::new();
        shiplift.run_with_args(image, run_args);

        //TODO add assert maybe receive the container and then inspect the id
    }
}
