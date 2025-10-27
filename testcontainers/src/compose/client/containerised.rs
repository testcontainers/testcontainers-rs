use std::path::PathBuf;

use crate::{
    compose::{
        client::{ComposeInterface, DownCommand, UpCommand},
        error::Result,
    },
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
    pub(super) async fn new(compose_files: Vec<PathBuf>) -> Result<Self> {
        let mut image = ContainerRequest::from(DockerCli::new("/var/run/docker.sock"));

        let compose_files_in_container: Vec<String> = compose_files
            .iter()
            .enumerate()
            .map(|(i, _)| format!("/docker-compose-{i}.yml"))
            .collect();
        let mounts: Vec<_> = compose_files
            .iter()
            .zip(compose_files_in_container.iter())
            .filter_map(|(path, file_name)| path.to_str().map(|p| Mount::bind_mount(p, file_name)))
            .collect();

        for mount in mounts {
            image = image.with_mount(mount);
        }

        let container = image.start().await?;

        Ok(Self {
            container,
            compose_files_in_container,
        })
    }
}

impl ComposeInterface for ContainerisedComposeCli {
    async fn up(&self, command: UpCommand) -> Result<()> {
        let mut cmd_parts = vec![];

        for (key, value) in &command.env_vars {
            cmd_parts.push(format!("{}={}", key, value));
        }

        cmd_parts.extend([
            "docker".to_string(),
            "compose".to_string(),
            "--project-name".to_string(),
            command.project_name.clone(),
        ]);

        for file in &self.compose_files_in_container {
            cmd_parts.push("-f".to_string());
            cmd_parts.push(file.to_string());
        }

        cmd_parts.push("up".to_string());

        if command.build {
            cmd_parts.push("--build".to_string());
        }

        if command.pull {
            cmd_parts.push("--pull".to_string());
            cmd_parts.push("always".to_string());
        }

        cmd_parts.push("--wait".to_string());
        cmd_parts.push("--wait-timeout".to_string());
        cmd_parts.push(command.wait_timeout.as_secs().to_string());

        let exec = ExecCommand::new(cmd_parts);
        self.container.exec(exec).await?;

        Ok(())
    }

    async fn down(&self, command: DownCommand) -> Result<()> {
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
        self.container.exec(exec).await?;
        Ok(())
    }
}
