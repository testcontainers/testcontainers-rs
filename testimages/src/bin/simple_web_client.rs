use std::{
    env::var,
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

fn main() {
    let server_ip = var("SERVER_IP").unwrap();
    let server_port = var("SERVER_PORT").unwrap();
    let server_address = format!("{server_ip}:{server_port}");
    let connection_timeout = Duration::from_secs(3);

    let mut socket_addresses = server_address.to_socket_addrs().unwrap();
    let socket_addr = socket_addresses.next().unwrap();

    println!("Attempting connection.");

    let connection = match TcpStream::connect_timeout(&socket_addr, connection_timeout) {
        Ok(connection) => connection,
        Err(err) => {
            eprintln!("{err}");
            panic!();
        }
    };

    println!("Client connected.");

    dbg!(connection);
}
