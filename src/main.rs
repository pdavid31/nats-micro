mod services;

use std::error::Error as StdError;

extern crate tokio;

use anyhow::Error;
use async_nats::{
    service::{Request, ServiceExt},
    ConnectOptions,
};
use bytes::Bytes;
use futures::StreamExt;
use tokio_stream::StreamMap;

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    let address = "localhost:4222";
    let options = ConnectOptions::new()
        .name("svc.math")
        .event_callback(|event| async move {
            match event {
                async_nats::Event::Disconnected => println!("disconnected"),
                async_nats::Event::Connected => println!("reconnected"),
                async_nats::Event::ClientError(err) => println!("client error occurred: {}", err),
                other => println!("other event happened: {}", other),
            }
        });

    let client = async_nats::connect_with_options(address, options).await?;

    let service = client
        .service_builder()
        .description("This is a great math microservice")
        .queue_group("svc.math")
        .start("math", "0.1.0")
        .await?;

    let v1_group = service.group("svc.math.v1");

    let mut stream_map = StreamMap::new();
    stream_map.insert("svc.math.v1.add", v1_group.endpoint("add").await?);

    println!("Starting to listen on {}", address);
    while let Some((key, request)) = stream_map.next().await {
        match key {
            "svc.math.v1.add" => wrapper(services::add, &request).await?,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn wrapper<'a, F, I, O>(func: F, request: &'a Request) -> Result<(), async_nats::PublishError>
where
    F: FnOnce(I) -> Result<O, Error>,
    I: TryFrom<&'a Request>,
    O: TryInto<Bytes>,
    // we have to make sure that the conversion from input and output to bytes
    // produces printable errors, which is why require those to implement std::error::Error
    <I as TryFrom<&'a Request>>::Error: StdError,
    <O as TryInto<Bytes>>::Error: StdError,
{
    let response = internal_caller(func, request).await;
    request.respond(response).await
}

async fn internal_caller<'a, F, I, O>(
    func: F,
    request: &'a Request,
) -> Result<Bytes, async_nats::service::error::Error>
where
    F: FnOnce(I) -> Result<O, Error>,
    I: TryFrom<&'a Request>,
    O: TryInto<Bytes>,
    // we have to make sure that the conversion from input and output to bytes
    // produces printable errors, which is why require those to implement std::error::Error
    <I as TryFrom<&'a Request>>::Error: StdError,
    <O as TryInto<Bytes>>::Error: StdError,
{
    let input = I::try_from(request).map_err(|err| async_nats::service::error::Error {
        status: format!("failed to deserialize request body: {}", err),
        code: 400,
    })?;

    let result = func(input).map_err(|err| async_nats::service::error::Error {
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
