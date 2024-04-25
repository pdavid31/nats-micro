use std::{
    error::Error as StdError,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

extern crate pin_project;

use async_nats::service::endpoint::Endpoint;
use futures::Stream;

use crate::handler::{Handler, HandlerExt as _};

// TODO: there as to be a better way to this than repeating the Input and Output requirements here
#[pin_project::pin_project]
pub struct EndpointHandler<T, Fut>
where
    T: Handler,
    Fut: Future<Output = Result<(), async_nats::PublishError>>,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    handler: T,

    #[pin]
    endpoint: Endpoint,

    #[pin]
    future: Option<Fut>,
}

// TODO: when running this, futures will block indefinetly
// NOTE: implementation inspired by https://docs.rs/futures-util/0.3.30/src/futures_util/stream/stream/then.rs.html
impl<T, Fut> Stream for EndpointHandler<T, Fut>
where
    T: Handler,
    Fut: Future<Output = Result<(), async_nats::PublishError>>,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    type Item = Fut::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // Check if we currently have a future stored.
            // If that's the case, we try to drive this to completion.
            // Otherwise, we poll the underlying stream for a new request.
            // If none of those options currently has a value,
            // we just return None
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let response = futures::ready!(fut.poll(cx));
                this.future.set(None);

                return Poll::Ready(Some(response));
            } else if let Some(item) = ready!(this.endpoint.as_mut().poll_next(cx)) {
                let x = this.handler.handle(item);
                this.future.set(Some(x));

                continue;
            } else {
                return Poll::Ready(None);
            }
        }
    }
}

pub trait EndpointWithHandler<T, Fut>
where
    T: Handler,
    Fut: Future<Output = Result<(), async_nats::PublishError>>,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<T, Fut>, anyhow::Error>;
}

impl<T, Fut> EndpointWithHandler<T, Fut> for Endpoint
where
    T: Handler,
    Fut: Future<Output = Result<(), async_nats::PublishError>>,
    <T::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<T, Fut>, anyhow::Error> {
        Ok(EndpointHandler {
            endpoint: self,
            handler,

            future: None,
        })
    }
}
