mod endpoint;
mod handler;
mod services;

extern crate tokio;

use async_nats::{service::ServiceExt, ConnectOptions};
use tokio_stream::{StreamExt as _, StreamMap};

use crate::{endpoint::EndpointWithHandler as _, services::AddHandler};

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

    let handler = AddHandler::new();
    let endpoint_handler = v1_group.endpoint("add").await?.with_handler(handler)?;

    stream_map.insert("svc.math.v1.add", endpoint_handler);

    println!("Starting to listen on {}", address);
    while let Some((key, result)) = stream_map.next().await {
        match result {
            Ok(_) => {}
            Err(err) => {
                println!("encountered error on {}: {}", key, err);
            }
        }
    }

    Ok(())
}
