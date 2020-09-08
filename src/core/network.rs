use crate::Docker;

/// Represents a docker network.
///
/// Networks have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let docker = clients::Cli::default();
///
///     {
///         let network = docker.create_network(NetworkConfig::new("test-net"));
///
///         // Docker network is removed at the end of this scope.
///     }
/// }
///
/// ```
///
/// [drop_impl]: struct.Network.html#impl-Drop
#[derive(Debug)]
pub struct Network<'d, D>
where
    D: Docker,
{
    id: String,
    name: String,
    docker_client: &'d D,
}

impl<'d, D> Network<'d, D>
where
    D: Docker,
{
    /// Constructs a new network given a name and a docker client.
    pub fn new(id: String, name: String, docker_client: &'d D) -> Self {
        Network {
            id,
            name,
            docker_client,
        }
    }

    /// Returns the id of this network.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rm(&self) {
        log::debug!("Deleting docker networtk {}", self.name);

        self.docker_client.rm_network(&self.name)
    }
}

/// The destructor implementation for a Network.
///
/// As soon as the network goes out of scope, the destructor will delete the docker network.
impl<'d, D> Drop for Network<'d, D>
where
    D: Docker,
{
    fn drop(&mut self) {
        self.rm()
    }
}

#[derive(Debug)]
pub struct NetworkConfig {
    pub name: String,
}

impl NetworkConfig {
    pub fn new(name: String) -> Self {
        NetworkConfig { name }
    }
}
