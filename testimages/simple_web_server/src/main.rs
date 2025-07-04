//! A basic HTTP server, to test overriding a container's ENTRYPOINT.
use std::{env, net::SocketAddr, path::PathBuf};

use axum::{routing::get, Router};
use tokio::signal;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/", get(handler));

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    eprintln!("server will be listening to the port 80");
    println!("server is ready");
    println!("server is ready"); // duplicate line to test `times` parameter of `WaitFor::Log`
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn handler() -> String {
    let argv_0: PathBuf = env::args_os().next().unwrap().into();
    argv_0.file_name().unwrap().to_str().unwrap().to_string()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
