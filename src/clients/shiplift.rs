use crate::core::{ContainerAsync, DockerAsync, ImageAsync, LogsAsync, Ports, RunArgs};
use async_trait::async_trait;
use futures::stream::StreamExt;
use shiplift::Docker;
use shiplift::{ContainerOptions, LogsOptions, NetworkCreateOptions, RmContainerOptions};
use std::fmt;

pub struct Shiplift {
    client: Docker,
}

impl fmt::Debug for Shiplift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shiplift").finish()
    }
}

impl Shiplift {
    pub fn new() -> Self {
        return Shiplift {
            client: Docker::new(),
        };
    }
}

#[async_trait]
impl DockerAsync for Shiplift {
    async fn run<I: ImageAsync + Sync>(&self, image: I) -> ContainerAsync<'_, Shiplift, I> {
        let empty_args = RunArgs::default();
        self.run_with_args(image, empty_args).await
    }

    async fn run_with_args<I: ImageAsync + Send + Sync>(
        &self,
        image: I,
        run_args: RunArgs,
    ) -> ContainerAsync<'_, Shiplift, I> {
        // Create network
        if let Some(network) = run_args.network() {
            self.client
                .networks()
                .create(&NetworkCreateOptions::builder(network.as_str()).build())
                .await
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
            .client
            .containers()
            .create(&options_builder.build())
            .await;
        let id = create_result.unwrap().id;
        self.client.containers().get(&id).start().await.unwrap();

        ContainerAsync::new(id, self, image).await
    }

    // static str here might not be the best option
    async fn logs<'a>(&'a self, id: &'a str) -> LogsAsync<'a> {
        // XXX Need advice to handle these two unwrap
        let logs_stream_stdout = self
            .client
            .containers()
            .get(id)
            .logs(&LogsOptions::builder().stdout(true).stderr(false).build())
            .map(|chunk| chunk.unwrap().into());

        let logs_stream_stderr = self
            .client
            .containers()
            .get(id)
            .logs(&LogsOptions::builder().stdout(false).stderr(true).build())
            .map(|chunk| chunk.unwrap().into());

        LogsAsync {
            stdout: Box::new(logs_stream_stdout),
            stderr: Box::new(logs_stream_stderr),
        }
    }

    async fn ports(&self, id: &str) -> Ports {
        let mut ports = Ports::default();
        let container_detatils = self.client.containers().get(id).inspect().await.unwrap();

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

    async fn rm(&self, id: &str) {
        self.client
            .containers()
            .get(id)
            .remove(
                RmContainerOptions::builder()
                    .volumes(true)
                    .force(true)
                    .build(),
            )
            .await
            .unwrap();
    }

    async fn stop(&self, id: &str) {
        self.client
            .containers()
            .get(id)
            .stop(Option::None)
            .await
            .unwrap();
    }

    async fn start(&self, id: &str) {
        self.client.containers().get(id).start().await.unwrap();
    }
}

fn parse_port(port: &str) -> u16 {
    port.parse()
        .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
}

#[cfg(test)]
mod tests {
    use crate::core::Port;
    use crate::{core::ContainerAsync, core::DockerAsync, core::ImageAsync};
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct HelloWorld {
        volumes: HashMap<String, String>,
        env_vars: HashMap<String, String>,
    }

    #[async_trait]
    impl ImageAsync for HelloWorld {
        type Args = Vec<String>;
        type EnvVars = HashMap<String, String>;
        type Volumes = HashMap<String, String>;
        type EntryPoint = std::convert::Infallible;

        fn descriptor(&self) -> String {
            String::from("hello-world")
        }

        async fn wait_until_ready<D: DockerAsync + Sync>(
            &self,
            _container: &ContainerAsync<'_, D, Self>,
        ) {
        }

        fn args(&self) -> <Self as ImageAsync>::Args {
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

        fn with_args(self, _arguments: <Self as ImageAsync>::Args) -> Self {
            self
        }
    }

    #[tokio::test]
    async fn shiplift_can_run_container() {
        let image = HelloWorld::default();
        let shiplift = Shiplift::new();
        shiplift.run(image).await;
    }
}
