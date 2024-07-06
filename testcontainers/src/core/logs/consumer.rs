use futures::{future::BoxFuture, FutureExt};

use crate::core::logs::LogFrame;

pub mod logging_consumer;

/// Log consumer is a trait that allows to consume log frames.
/// Consumers will be called for each log frame that is produced by the container for the whole lifecycle of the container.
pub trait LogConsumer: Send + Sync {
    fn accept<'a>(&'a self, record: &'a LogFrame) -> BoxFuture<'a, ()>;
}

impl<F> LogConsumer for F
where
    F: Fn(&LogFrame) + Send + Sync,
{
    fn accept<'a>(&'a self, record: &'a LogFrame) -> BoxFuture<'a, ()> {
        // preferably to spawn blocking task
        async move {
            self(record);
        }
        .boxed()
    }
}
