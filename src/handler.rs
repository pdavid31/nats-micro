use std::error::Error as StdError;

extern crate anyhow;
extern crate async_nats;
extern crate bytes;

/// The handler trait allows us to easily implement our MicroService endpoints
/// using plain types
pub trait Handler<'a>
where
    <Self::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <Self::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    // Input is required to implement TryFrom<Request>, where
    // the associated error type must implement StdError (see up)
    type Input: TryFrom<&'a async_nats::service::Request>;

    // Out is required to implement TryInto<Bytes>, where
    // the associated error type must implement StdError (see up)
    type Output: TryInto<bytes::Bytes>;

    async fn compute(&self, user_input: Self::Input) -> Result<Self::Output, anyhow::Error>;
}

/// Using the HandlerExt trait, we easily define shared behaviour between
/// our Handlers
pub trait HandlerExt<'a>: Handler<'a>
where
    <Self::Input as TryFrom<&'a async_nats::service::Request>>::Error: StdError,
    <Self::Output as TryInto<bytes::Bytes>>::Error: StdError,
{
    // Handle incoming requests by feeding the underlying
    // compute function. This is the most top level function to be
    // called.
    async fn handle(
        &self,
        request: &'a async_nats::service::Request,
    ) -> Result<(), async_nats::PublishError> {
        // call the internal procedure
        // every outcome (Ok or Error) will be handled and
        // reported back to the user
        let response = self.internal_compute(request).await;
        request.respond(response).await
    }

    // Automatically handle decoding of the user request,
    // encoding of the generated response and error mapping
    async fn internal_compute(
        &self,
        request: &'a async_nats::service::Request,
    ) -> Result<bytes::Bytes, async_nats::service::error::Error> {
        let input =
            Self::Input::try_from(request).map_err(|err| async_nats::service::error::Error {
                status: format!("failed to deserialize request body: {}", err),
                code: 400,
            })?;

        let result =
            self.compute(input)
                .await
                .map_err(|err| async_nats::service::error::Error {
                    status: format!("failed to compute: {}", err),
                    code: 500,
                })?;

        result
            .try_into()
            .map_err(|err| async_nats::service::error::Error {
                status: format!("failed serialize response body: {}", err),
                code: 500,
            })
    }
}
