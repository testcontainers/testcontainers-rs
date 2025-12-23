use crate::{
    compose::{
        client::{ComposeInterface, DownCommand, UpCommand},
        error::Result,
        ContainerisedComposeOptions,
    },
    core::{CmdWaitFor, ExecCommand},
    images::docker_cli::DockerCli,
    runners::AsyncRunner,
    ContainerAsync, ContainerRequest, ImageExt,
};

pub(crate) struct ContainerisedComposeCli {
    container: ContainerAsync<DockerCli>,
    compose_files_in_container: Vec<String>,
    project_directory: Option<String>,
}

impl ContainerisedComposeCli {
    pub(super) async fn new(options: ContainerisedComposeOptions) -> Result<Self> {
        let (compose_files, project_directory) = options.into_parts();
        let mut image = ContainerRequest::from(DockerCli::new("/var/run/docker.sock"));

        let compose_files_in_container: Vec<String> = compose_files
            .iter()
            .enumerate()
            .map(|(i, _)| format!("/docker-compose-{i}.yml"))
            .collect();
        for (path, file_name) in compose_files
            .into_iter()
            .zip(compose_files_in_container.iter())
        {
            image = image.with_copy_to(file_name, path);
        }

        let container = image.start().await?;
        let project_directory = project_directory.map(|path| path.to_string_lossy().into_owned());

        Ok(Self {
            container,
            compose_files_in_container,
            project_directory,
        })
    }
}

impl ComposeInterface for ContainerisedComposeCli {
    async fn up(&self, command: UpCommand) -> Result<()> {
        let mut cmd_parts = vec![];

        for (key, value) in &command.env_vars {
            cmd_parts.push(format!("{}={}", key, value));
        }

        cmd_parts.extend(["docker".to_string(), "compose".to_string()]);

        if let Some(project_directory) = &self.project_directory {
            cmd_parts.push("--project-directory".to_string());
            cmd_parts.push(project_directory.clone());
        }

        cmd_parts.push("--project-name".to_string());
        cmd_parts.push(command.project_name.clone());

        for file in &self.compose_files_in_container {
            cmd_parts.push("-f".to_string());
            cmd_parts.push(file.to_string());
        }

        cmd_parts.push("up".to_string());
        cmd_parts.push("-d".to_string());

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

        let exec = ExecCommand::new(cmd_parts).with_cmd_ready_condition(CmdWaitFor::exit_code(0));
        self.container.exec(exec).await?;

        Ok(())
    }

    async fn down(&self, command: DownCommand) -> Result<()> {
        let mut cmd = vec!["docker".to_string(), "compose".to_string()];

        if let Some(project_directory) = &self.project_directory {
            cmd.push("--project-directory".to_string());
            cmd.push(project_directory.clone());
        }

        cmd.extend([
            "--project-name".to_string(),
            command.project_name.clone(),
            "down".to_string(),
        ]);

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
