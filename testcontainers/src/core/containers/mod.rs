pub(crate) mod async_container;
#[cfg(feature = "blocking")]
pub(crate) mod sync_container;

pub use async_container::{exec::ExecResult, ContainerAsync};
#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub use sync_container::{exec::SyncExecResult, Container};
