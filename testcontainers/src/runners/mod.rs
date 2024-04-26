pub(crate) mod async_runner;
#[cfg(feature = "blocking")]
pub(crate) mod sync_runner;

pub use self::async_runner::AsyncRunner;
#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub use self::sync_runner::SyncRunner;
