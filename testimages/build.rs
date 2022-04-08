use std::{env, process::Command};

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let cwd = env::var("CARGO_MANIFEST_DIR")?;

    // Build the test images in the repository
    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(&format!("{cwd}/src/dockerfiles/expose_port.dockerfile"))
        .arg("--force-rm")
        .arg("--tag")
        .arg("expose_port:latest")
        .arg(".")
        .output()?;
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8(output.stderr)?);
        bail!("unable to build expose_port:latest");
    }
    eprintln!("Built expose_port:latest");

    // trigger recompilation when dockerfiles are modified
    println!("cargo:rerun-if-changed=src/dockerfiles");
    println!("cargo:rerun-if-changed=.dockerignore");

    Ok(())
}
