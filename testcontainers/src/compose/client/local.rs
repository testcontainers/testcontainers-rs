use std::path::{Path, PathBuf};

use crate::compose::{
    client::{ComposeInterface, DownCommand, UpCommand},
    error::{ComposeError, Result},
};

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
        compose_files
            .first()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| Path::new("."))
    }
}

impl ComposeInterface for LocalComposeCli {
    async fn up(&self, command: UpCommand) -> Result<()> {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.current_dir(self.working_dir.as_path())
            .arg("compose")
            .arg("--project-name")
            .arg(&command.project_name);

        for compose_file in &self.compose_files {
            cmd.arg("-f").arg(compose_file);
        }

        cmd.arg("up");

        if command.build {
            cmd.arg("--build");
        }

        if command.pull {
            cmd.arg("--pull").arg("always");
        }

        cmd.arg("--wait")
            .arg("--wait-timeout")
            .arg(command.wait_timeout.as_secs().to_string());

        for (key, value) in &command.env_vars {
            cmd.env(key, value);
        }

        cmd.output()
            .await
            .map_err(|e| ComposeError::Testcontainers(e.into()))?;

        Ok(())
    }

    async fn down(&self, command: DownCommand) -> Result<()> {
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

        cmd.output()
            .await
            .map_err(|e| ComposeError::Testcontainers(e.into()))?;

        Ok(())
    }
}
