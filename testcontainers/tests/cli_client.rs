use testcontainers::{core::WaitFor, *};

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    type Args = ();

    fn name(&self) -> String {
        "hello-world".to_owned()
    }

    fn tag(&self) -> String {
        "latest".to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Hello from Docker!")]
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cli_can_run_hello_world() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Cli::default();

    let _container = docker.run(HelloWorld);
}

#[test]
fn generic_image_with_custom_entrypoint() {
    let docker = clients::Cli::default();
    let msg = WaitFor::message_on_stdout("server is ready");

    let generic = GenericImage::new("simple_web_server", "latest").with_wait_for(msg.clone());

    let node = docker.run(generic);
    let port = node.get_host_port_ipv4(80);
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://127.0.0.1:{port}"))
            .unwrap()
            .text()
            .unwrap()
    );

    let generic = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(msg)
        .with_entrypoint("./bar");

    let node = docker.run(generic);
    let port = node.get_host_port_ipv4(80);
    assert_eq!(
        "bar",
        reqwest::blocking::get(format!("http://127.0.0.1:{port}"))
            .unwrap()
            .text()
            .unwrap()
    );
}

#[test]
fn generic_image_exposed_ports() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();

    let target_port = 8080;

    // This server does not EXPOSE ports in its image.
    let generic_server = GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"))
        // Explicitly expose the port, which otherwise would not be available.
        .with_exposed_port(target_port);

    let node = docker.run(generic_server);
    let port = node.get_host_port_ipv4(target_port);
    assert!(reqwest::blocking::get(format!("http://127.0.0.1:{port}"))
        .unwrap()
        .status()
        .is_success());
}

#[test]
#[should_panic]
fn generic_image_port_not_exposed() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();

    let target_port = 8080;

    // This image binds to 0.0.0.0:8080, does not EXPOSE ports in its dockerfile.
    let generic_server = GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"));
    let node = docker.run(generic_server);

    // Without exposing the port with `with_exposed_port()`, we cannot get a mapping to it.
    node.get_host_port_ipv4(target_port);
}
