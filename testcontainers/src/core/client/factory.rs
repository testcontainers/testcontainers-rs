use std::sync::{Arc, OnceLock, Weak};

use tokio::sync::Mutex;

use crate::core::client::{Client, ClientError};

// We use `Weak` in order not to prevent `Drop` of being called.
// Instead, we re-create the client if it was dropped and asked one more time.
// This way we provide on `Drop` guarantees and avoid unnecessary instantiation at the same time.
static DOCKER_CLIENT: OnceLock<Mutex<Weak<Client>>> = OnceLock::new();

impl Client {
    /// Returns a client instance, reusing already created or initializing a new one.
    pub(crate) async fn lazy_client() -> Result<Arc<Client>, ClientError> {
        let mut guard = DOCKER_CLIENT
            .get_or_init(|| Mutex::new(Weak::new()))
            .lock()
            .await;
        let maybe_client = guard.upgrade();

        if let Some(client) = maybe_client {
            Ok(client)
        } else {
            let client = Arc::new(Client::new().await?);
            *guard = Arc::downgrade(&client);

            Ok(client)
        }
    }
}

/// Returns a configured Docker client instance.
///
/// This function provides access to the underlying Docker client ([`bollard`]).
/// While this method is publicly exposed, it is not intended for frequent use.
/// It can be useful in scenarios where you need to interact with the Docker API directly using an already configured client.
///
/// This method returns a lazily-created client, reusing an existing one if available.
pub async fn docker_client_instance() -> Result<bollard::Docker, ClientError> {
    Client::lazy_client().await.map(|c| c.bollard.clone())
}
