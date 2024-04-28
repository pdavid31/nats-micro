use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use async_nats::{service::endpoint::Endpoint, PublishError};
use futures::Stream;
use pin_project_lite::pin_project;

use crate::{Handler, HandlerExt};

pin_project! {
    pub struct EndpointHandler<T, Fut> {
        #[pin]
        endpoint: Endpoint,
        #[pin]
        handler: T,
        #[pin]
        future: Option<Fut>,
    }
}

impl<T, Fut> Stream for EndpointHandler<T, Fut>
where
    T: Handler,
    Fut: Future<Output = Result<(), PublishError>>,
{
    type Item = Fut::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let result = futures::ready!(fut.poll(cx));
                this.future.set(None);

                return Poll::Ready(Some(result));
            } else if let Some(request) = futures::ready!(this.endpoint.as_mut().poll_next(cx)) {
                let future = this.handler.handle(request);
                // TODO: why is this mf'er not working?
                // this.future.set(Some(future));

                continue;
            } else {
                return Poll::Ready(None);
            }
        }
    }
}

pub trait EndpointWithHandler<T, Fut> {
    fn with_handler(self, handler: T) -> EndpointHandler<T, Fut>;
}

impl<T, Fut> EndpointWithHandler<T, Fut> for Endpoint
where
    T: Handler,
    Fut: Future<Output = Result<(), PublishError>>,
{
    fn with_handler(self, handler: T) -> EndpointHandler<T, Fut> {
        EndpointHandler {
            endpoint: self,
            handler,
            future: None,
        }
    }
}
