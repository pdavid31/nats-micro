use std::{error::Error as StdError, sync::Arc};

extern crate anyhow;
extern crate async_nats;
extern crate bytes;

/// The handler trait allows us to easily implement our MicroService endpoints
/// using plain types
pub trait Handler
where
    <Self::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <Self::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    // Input is required to implement TryFrom<Request>, where
    // the associated error type must implement StdError (see up)
    type Input: TryFrom<Arc<async_nats::service::Request>>;

    // Out is required to implement TryInto<Bytes>, where
    // the associated error type must implement StdError (see up)
    type Output: TryInto<bytes::Bytes>;

    // TODO: we should introduce a custom error type here instead of anyhow::Error
    fn compute(
        &self,
        user_input: Self::Input,
    ) -> impl std::future::Future<Output = Result<Self::Output, anyhow::Error>> + Send;
}

/// Using the HandlerExt trait, we easily define shared behaviour between
/// our Handlers
pub(crate) trait HandlerExt: Handler
where
    <Self::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <Self::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    // Handle incoming requests by feeding the underlying
    // compute function. This is the most top level function to be
    // called.
    async fn handle(
        &self,
        request: async_nats::service::Request,
    ) -> Result<(), async_nats::PublishError> {
        let request_arc = Arc::new(request);
        // call the internal procedure
        // every outcome (Ok or Error) will be handled and
        // reported back to the user
        let response = self.internal_compute(request_arc.clone()).await;
        request_arc.respond(response).await
    }

    // Automatically handle decoding of the user request,
    // encoding of the generated response and error mapping
    async fn internal_compute(
        &self,
        request: Arc<async_nats::service::Request>,
    ) -> Result<bytes::Bytes, async_nats::service::error::Error> {
        // Try to construct our input object
        // from the NATS Service Request object
        let input =
            Self::Input::try_from(request).map_err(|err| async_nats::service::error::Error {
                status: format!("failed to deserialize request body: {}", err),
                code: 400,
            })?;

        // call the associated compute function
        let result =
            self.compute(input)
                .await
                .map_err(|err| async_nats::service::error::Error {
                    status: format!("failed to compute: {}", err),
                    code: 500,
                })?;

        // try to convert the result into Bytes object,
        // allowing to easily write it back to NATS
        result
            .try_into()
            .map_err(|err| async_nats::service::error::Error {
                status: format!("failed to serialize response body: {}", err),
                code: 500,
            })
    }
}

// Implement the HandlerExt trait for every type that implements Handler
impl<T> HandlerExt for T
where
    T: Handler,
    <Self::Input as TryFrom<Arc<async_nats::service::Request>>>::Error: StdError,
    <Self::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
}
