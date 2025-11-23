use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    core::{mounts::Mount, WaitFor},
    Image,
};

const IMAGE_NAME: &str = "docker";
const IMAGE_TAG: &str = "26.1-cli";

pub(crate) struct DockerCli {
    envs: BTreeMap<String, String>,
    mounts: Vec<Mount>,
}

impl DockerCli {
    pub(crate) fn new(docker_socket: &str) -> Self {
        let mounts = vec![Mount::bind_mount(docker_socket, "/docker.sock")];
        let envs = BTreeMap::from([("DOCKER_HOST".to_string(), "unix:///docker.sock".to_string())]);

        DockerCli { envs, mounts }
    }
}

impl Image for DockerCli {
    fn name(&self) -> &str {
        IMAGE_NAME
    }

    fn tag(&self) -> &str {
        IMAGE_TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        Box::new(self.envs.iter())
    }

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        Box::new(self.mounts.iter())
    }

    fn entrypoint(&self) -> Option<&str> {
        Some("/bin/sh")
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        // keep container alive until dropped
        ["-c", "sleep infinity"]
    }
}
