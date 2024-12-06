use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock, Weak},
};

use tokio::sync::Mutex;

use crate::core::{
    async_drop,
    client::{Client, ClientError},
    env,
};

pub(crate) static CREATED_NETWORKS: OnceLock<Mutex<HashMap<String, Weak<Network>>>> =
    OnceLock::new();

fn created_networks() -> &'static Mutex<HashMap<String, Weak<Network>>> {
    CREATED_NETWORKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) struct Network {
    name: String,
    id: String,
    client: Arc<Client>,
}

impl Network {
    pub(crate) async fn new(
        name: impl Into<String>,
        client: Arc<Client>,
    ) -> Result<Option<Arc<Self>>, ClientError> {
        let name = name.into();
        let mut guard = created_networks().lock().await;
        let network = if let Some(network) = guard.get(&name).and_then(Weak::upgrade) {
            network
        } else {
            if client.network_exists(&name).await? {
                // Networks already exists and created outside the testcontainers
                return Ok(None);
            }

            let id = client.create_network(&name).await?;

            let created = Arc::new(Self {
                name: name.clone(),
                id,
                client,
            });

            guard.insert(name, Arc::downgrade(&created));

            created
        };

        Ok(Some(network))
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        if self.client.config.command() == env::Command::Remove {
            let client = self.client.clone();
            let name = self.name.clone();

            let drop_task = async move {
                log::trace!("Drop was called for network {name}, cleaning up");
                let mut guard = created_networks().lock().await;

                // check the strong count under the lock to avoid any possible race-conditions.
                let is_network_in_use = guard
                    .get(&name)
                    .filter(|weak| weak.strong_count() > 0)
                    .is_some();

                if is_network_in_use {
                    log::trace!("Network {name} was not dropped because it is still in use");
                } else {
                    guard.remove(&name);
                    match client.remove_network(&name).await {
                        Ok(_) => {
                            log::trace!("Network {name} was successfully dropped");
                        }
                        Err(_) => {
                            log::error!("Failed to remove network {name} on drop");
                        }
                    }
                }
            };

            async_drop::async_drop(drop_task);
        }
    }
}

impl fmt::Debug for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Network")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}
