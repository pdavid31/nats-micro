use std::{
    error::Error as StdError,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use async_nats::service::endpoint::Endpoint;
use futures::{Future, FutureExt, Stream, StreamExt};

use crate::handler::{Handler, HandlerExt as _};

// TODO: there as to be a better way to this than repeating the Input and Output requirements here
pub struct EndpointHandler<'a, T>
where
    T: Handler<'a>,
    <T::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    endpoint: Endpoint,
    handler: T,

    phantom: PhantomData<&'a T>,
}

impl<'a, T> Stream for EndpointHandler<'a, T>
where
    T: Handler<'a> + Unpin,
    <T::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    type Item = Result<(), async_nats::PublishError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let internal_poll = self.endpoint.poll_next_unpin(cx);

        match internal_poll {
            Poll::Ready(Some(request)) => {
                let mut fut = Box::pin(self.handler.handle(&request));

                fut.poll_unpin(cx).map(|x| Some(x))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait EndpointWithHandler<'a, T>
where
    T: Handler<'a>,
    <T::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<'a, T>, anyhow::Error>;
}

impl<'a, T> EndpointWithHandler<'a, T> for Endpoint
where
    T: Handler<'a>,
    <T::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <T::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    fn with_handler(self, handler: T) -> Result<EndpointHandler<'a, T>, anyhow::Error> {
        Ok(EndpointHandler {
            endpoint: self,
            handler,

            phantom: PhantomData,
        })
    }
}
