use std::{io::Error, path::PathBuf};

use crate::{
    compose::client::{ComposeInterface, DownCommand, UpCommand},
    core::{CmdWaitFor, ExecCommand, Mount},
    images::docker_cli::DockerCli,
    runners::AsyncRunner,
    ContainerAsync, ContainerRequest, ImageExt,
};

pub(crate) struct ContainerisedComposeCli {
    container: ContainerAsync<DockerCli>,
    compose_files_in_container: Vec<String>,
}

impl ContainerisedComposeCli {
    pub(super) async fn new(compose_files: Vec<PathBuf>) -> Self {
        let mut image = ContainerRequest::from(DockerCli::new("/var/run/docker.sock"));

        let compose_files_in_container: Vec<String> = compose_files
            .iter()
            .enumerate()
            .map(|(i, _)| format!("/docker-compose-{i}.yml"))
            .collect();
        let mounts: Vec<_> = compose_files
            .iter()
            .zip(compose_files_in_container.iter())
            .map(|(path, file_name)| Mount::bind_mount(path.to_str().unwrap(), file_name))
            .collect();

        for mount in mounts {
            image = image.with_mount(mount);
        }

        let container = image.start().await.expect("TODO: Handle error");

        Self {
            container,
            compose_files_in_container,
        }
    }
}

impl ComposeInterface for ContainerisedComposeCli {
    async fn up(&self, command: UpCommand) -> Result<(), Error> {
        let mut cmd = vec![
            "docker".to_string(),
            "compose".to_string(),
            "--project-name".to_string(),
            command.project_name.clone(),
        ];

        for file in &self.compose_files_in_container {
            cmd.push("-f".to_string());
            cmd.push(file.to_string());
        }

        cmd.push("up".to_string());
        cmd.push("--wait".to_string());
        // add timeout
        cmd.push("--wait-timeout".to_string());
        cmd.push(command.wait_timeout.as_secs().to_string());

        let exec = ExecCommand::new(cmd);
        // todo: error handling
        self.container.exec(exec).await.map_err(Error::other)?;

        Ok(())
    }

    async fn down(&self, command: DownCommand) -> Result<(), Error> {
        let mut cmd = vec![
            "docker".to_string(),
            "compose".to_string(),
            "--project-name".to_string(),
            command.project_name.clone(),
            "down".to_string(),
        ];

        if command.volumes {
            cmd.push("--volumes".to_string());
        }
        if command.rmi {
            cmd.push("--rmi".to_string());
        }

        let exec = ExecCommand::new(cmd).with_cmd_ready_condition(CmdWaitFor::exit_code(0));
        self.container.exec(exec).await.map_err(Error::other)?;
        Ok(())
    }
}
