use testcontainers::{core::WaitFor, *};

#[test]
fn generic_image() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();

    let db = "postgres-db-test";
    let user = "postgres-user-test";
    let password = "postgres-password-test";

    let generic_postgres = images::generic::GenericImage::new("postgres", "9.6-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_DB", db)
        .with_env_var("POSTGRES_USER", user)
        .with_env_var("POSTGRES_PASSWORD", password);

    let node = docker.run(generic_postgres);

    let connection_string = &format!(
        "postgres://{user}:{password}@127.0.0.1:{}/{db}",
        node.get_host_port_ipv4(5432)
    );
    let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

    let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: i32 = first_row.get(0);
    assert_eq!(first_column, 2);
}

#[test]
fn generic_image_with_custom_entrypoint() {
    let docker = clients::Cli::default();
    let msg = WaitFor::message_on_stdout("server is ready");

    let generic = images::generic::GenericImage::new("simple_web_server", "latest")
        .with_wait_for(msg.clone());

    let node = docker.run(generic);
    let port = node.get_host_port_ipv4(80);
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://127.0.0.1:{port}"))
            .unwrap()
            .text()
            .unwrap()
    );

    let generic = images::generic::GenericImage::new("simple_web_server", "latest")
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
    let docker = clients::Cli::docker();

    let target_port = 8080;

    // This server does not EXPOSE ports in its image.
    let generic_server = images::generic::GenericImage::new("no_expose_port", "latest")
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
    let docker = clients::Cli::docker();

    let target_port = 8080;

    // This image binds to 0.0.0.0:8080, does not EXPOSE ports in its dockerfile.
    let generic_server = images::generic::GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"));
    let node = docker.run(generic_server);

    // Without exposing the port with `with_exposed_port()`, we cannot get a mapping to it.
    node.get_host_port_ipv4(target_port);
}
