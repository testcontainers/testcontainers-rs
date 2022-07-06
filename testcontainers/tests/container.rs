use testcontainers::{clients::Cli, images::hello_world::HelloWorld};

#[test]
fn container_should_be_send() {
    let docker = Cli::default();
    let node = docker.run(HelloWorld {});
    need_send(node);
}

#[test]
fn container_should_be_sync() {
    let docker = Cli::default();
    let node = docker.run(HelloWorld {});
    need_sync(node);
}

fn need_send<T: Send>(_t: T) {}
fn need_sync<T: Sync>(_t: T) {}
