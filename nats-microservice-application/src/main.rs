mod services;

extern crate tokio;

use async_nats::{service::ServiceExt, ConnectOptions};
use tokio_stream::{StreamExt as _, StreamMap};

use nats_microservice_rs::EndpointWithHandler as _;

use crate::services::WeatherHandler;

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
        .description("This is a great weather microservice")
        .queue_group("svc.weather")
        .start("weather", "0.1.0")
        .await?;

    let v1_group = service.group("svc.weather");

    let mut stream_map = StreamMap::new();

    let handler = WeatherHandler::new();
    let endpoint_handler = v1_group.endpoint("v1").await?.with_handler(handler)?;

    stream_map.insert("svc.weather.v1.add", endpoint_handler);

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
