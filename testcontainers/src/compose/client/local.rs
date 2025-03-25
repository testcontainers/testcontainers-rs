use std::{
    io::Error,
    path::{Path, PathBuf},
};

use crate::compose::client::{ComposeInterface, DownCommand, UpCommand};

#[derive(Debug)]
pub(crate) struct LocalComposeCli {
    compose_files: Vec<PathBuf>,
    working_dir: PathBuf,
}

impl LocalComposeCli {
    pub(super) fn new(compose_files: Vec<PathBuf>) -> Self {
        let working_dir = Self::extract_current_dir(&compose_files).to_path_buf();

        Self {
            compose_files,
            working_dir,
        }
    }

    fn extract_current_dir(compose_files: &[PathBuf]) -> &Path {
        // TODO: error handling
        compose_files
            .first()
            .expect("At least one compose file is required")
            .parent()
            .expect("Compose file path must be absolute")
    }
}

impl ComposeInterface for LocalComposeCli {
    async fn up(&self, command: UpCommand) -> Result<(), Error> {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.current_dir(self.working_dir.as_path())
            .arg("compose")
            .arg("--project-name")
            .arg(&command.project_name);

        for compose_file in &self.compose_files {
            cmd.arg("-f").arg(compose_file);
        }
        cmd.arg("up")
            .arg("--wait")
            .arg("--wait-timeout")
            .arg(command.wait_timeout.as_secs().to_string());

        cmd.output().await?;

        Ok(())
    }

    async fn down(&self, command: DownCommand) -> Result<(), Error> {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.current_dir(self.working_dir.as_path())
            .arg("compose")
            .arg("--project-name")
            .arg(&command.project_name)
            .arg("down");

        if command.volumes {
            cmd.arg("--volumes");
        }
        if command.rmi {
            cmd.arg("--rmi");
        }

        cmd.output().await?;

        Ok(())
    }
}
