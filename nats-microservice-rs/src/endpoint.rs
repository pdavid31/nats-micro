use std::{
    error::Error as StdError,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

extern crate pin_project_lite;

use async_nats::service::endpoint::Endpoint;
use futures::{Future, Stream};

use crate::handler::{Handler, HandlerExt as _};

pin_project_lite::pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct EndpointHandler<T, Fut> {
        #[pin]
        endpoint: Endpoint,
        #[pin]
        future: Option<Fut>,

        handler: T,
    }
}

impl<T, Fut> EndpointHandler<T, Fut>
where
    T: Handler,
{
    pub fn new(endpoint: Endpoint, handler: T) -> Self {
        Self {
            endpoint,
            handler,
            future: None,
        }
    }
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
                let fut = this.handler.handle(item);
                let fut_opt: Option<Fut> = Some(fut);
                this.future.set(fut_opt);

                continue;
            } else {
                return Poll::Ready(None);
            }
        }
    }
}

pub trait EndpointWithHandler<T>
where
    T: Handler,
{
    type Output: Future;

    fn with_handler(self, handler: T) -> Result<EndpointHandler<T, Self::Output>, anyhow::Error>;
}

impl<T> EndpointWithHandler<T> for Endpoint
where
    T: Handler,
{
    type Output = Pin<Box<dyn Future<Output = Result<(), async_nats::PublishError>>>>;

    fn with_handler(self, handler: T) -> Result<EndpointHandler<T, Self::Output>, anyhow::Error> {
        Ok(EndpointHandler::new(self, handler))
    }
}
