pub(crate) mod async_runner;

#[cfg(feature = "buildkit")]
pub(crate) mod async_builder;

#[cfg(feature = "buildkit")]
#[cfg_attr(docsrs, doc(cfg(feature = "buildkit")))]
pub use self::{async_builder::AsyncBuilder, async_runner::AsyncRunner};

#[cfg(all(feature = "blocking", feature = "buildkit"))]
pub(crate) mod sync_builder;

#[cfg(all(feature = "blocking", feature = "buildkit"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "blocking", feature = "buildkit"))))]
pub use self::sync_builder::SyncBuilder;

#[cfg(feature = "blocking")]
pub(crate) mod sync_runner;

#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub use self::sync_runner::SyncRunner;
