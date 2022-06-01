use std::{env, process::Command};

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let cwd = env::var("CARGO_MANIFEST_DIR")?;

    // Build the test images in the repository
    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(&format!("{cwd}/src/dockerfiles/no_expose_port.dockerfile"))
        .arg("--force-rm")
        .arg("--tag")
        .arg("no_expose_port:latest")
        .arg(".")
        .output()?;
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8(output.stderr)?);
        bail!("unable to build no_expose_port:latest");
    }
    eprintln!("Built no_expose_port:latest");

    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(&format!(
            "{cwd}/src/dockerfiles/simple_web_server.dockerfile"
        ))
        .arg("--force-rm")
        .arg("--tag")
        .arg("simple_web_server:latest")
        .arg(".")
        .output()?;
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8(output.stderr)?);
        bail!("unable to build simple_web_server:latest");
    }
    eprintln!("Built simple_web_server:latest");

    // trigger recompilation when dockerfiles are modified
    println!("cargo:rerun-if-changed=src/dockerfiles");
    println!("cargo:rerun-if-changed=.dockerignore");

    Ok(())
}
