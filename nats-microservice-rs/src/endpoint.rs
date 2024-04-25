use std::{
    error::Error as StdError,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use async_nats::service::endpoint::Endpoint;
use futures::{FutureExt, Stream, StreamExt};

use crate::handler::{Handler, HandlerExt as _};

// TODO: there as to be a better way to this than repeating the Input and Output requirements here
pub struct EndpointHandler<T>
where
    T: Handler,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    endpoint: Endpoint,
    handler: T,
}

impl<T> Stream for EndpointHandler<T>
// TODO: when running this, futures will block indefinetly
where
    T: Handler + Unpin,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    type Item = Result<(), async_nats::PublishError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let internal_poll = self.endpoint.poll_next_unpin(cx);

        match internal_poll {
            Poll::Ready(Some(request)) => {
                let mut fut = Box::pin(self.handler.handle(request));

                fut.poll_unpin(cx).map(Option::Some)
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait EndpointWithHandler<T>
where
    T: Handler,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<T>, anyhow::Error>;
}

impl<T> EndpointWithHandler<T> for Endpoint
where
    T: Handler,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<T>, anyhow::Error> {
        Ok(EndpointHandler {
            endpoint: self,
            handler,
        })
    }
}
