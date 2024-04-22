use crate::core::client::Client;
use std::sync::{Arc, OnceLock, Weak};
use tokio::sync::Mutex;

// We use `Weak` in order not to prevent `Drop` of being called.
// Instead, we re-create the client if it was dropped and asked one more time.
// This way we provide on `Drop` guarantees and avoid unnecessary instantiation at the same time.
static DOCKER_CLIENT: OnceLock<Mutex<Weak<Client>>> = OnceLock::new();

impl Client {
    /// Returns a client instance, reusing already created or initializing a new one.
    // We don't expose this function to the public API for now. We can do it later if needed.
    pub(crate) async fn lazy_client() -> Arc<Client> {
        let mut guard = DOCKER_CLIENT
            .get_or_init(|| Mutex::new(Weak::new()))
            .lock()
            .await;
        let maybe_client = guard.upgrade();

        if let Some(client) = maybe_client {
            client
        } else {
            let client = Arc::new(Client::new().await);
            *guard = Arc::downgrade(&client);

            client
        }
    }
}
