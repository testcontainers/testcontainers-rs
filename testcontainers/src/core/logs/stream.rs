use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{stream::BoxStream, Stream, StreamExt};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::core::logs::LogFrame;

pub(crate) type RawLogStream = BoxStream<'static, Result<Bytes, io::Error>>;

pin_project_lite::pin_project! {
    pub(crate) struct LogStream {
        #[pin]
        inner: BoxStream<'static, Result<LogFrame, io::Error>>,
    }
}

impl LogStream {
    pub fn new(stream: BoxStream<'static, Result<LogFrame, io::Error>>) -> Self {
        Self { inner: stream }
    }

    /// Filters the log stream to only include stdout messages.
    pub(crate) fn into_stdout(self) -> RawLogStream {
        self.inner
            .filter_map(|record| async move {
                match record {
                    Ok(LogFrame::StdOut(bytes)) => Some(Ok(bytes)),
                    Ok(LogFrame::StdErr(_)) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .boxed()
    }

    /// Filters the log stream to only include stderr messages.
    pub(crate) fn into_stderr(self) -> RawLogStream {
        self.inner
            .filter_map(|record| async move {
                match record {
                    Ok(LogFrame::StdErr(bytes)) => Some(Ok(bytes)),
                    Ok(LogFrame::StdOut(_)) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .boxed()
    }

    /// Splits the log stream into two streams, one for stdout and one for stderr.
    pub(crate) async fn split(self) -> (RawLogStream, RawLogStream) {
        let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            macro_rules! handle_error {
                ($res:expr) => {
                    if let Err(err) = $res {
                        log::debug!(
                            "Receiver has been dropped, stop producing messages: {}",
                            err
                        );
                        break;
                    }
                };
            }
            let mut output = self;
            while let Some(chunk) = output.next().await {
                match chunk {
                    Ok(LogFrame::StdOut(message)) => {
                        handle_error!(stdout_tx.send(Ok(message)));
                    }
                    Ok(LogFrame::StdErr(message)) => {
                        handle_error!(stderr_tx.send(Ok(message)));
                    }
                    Err(err) => {
                        let err = Arc::new(err);
                        handle_error!(
                            stdout_tx.send(Err(io::Error::new(io::ErrorKind::Other, err.clone())))
                        );
                        handle_error!(
                            stderr_tx.send(Err(io::Error::new(io::ErrorKind::Other, err)))
                        );
                    }
                }
            }
        });

        let stdout = UnboundedReceiverStream::new(stdout_rx).boxed();
        let stderr = UnboundedReceiverStream::new(stderr_rx).boxed();
        (stdout, stderr)
    }
}

impl Stream for LogStream {
    type Item = Result<LogFrame, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}
