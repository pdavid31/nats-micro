mod services;

extern crate tokio;

use async_nats::{service::ServiceExt, ConnectOptions};
use tokio_stream::{StreamExt as _, StreamMap};

use nats_microservice_rs::HandlerExt;

use crate::services::{AddHandler, WeatherHandler};

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

    let mut stream_map = StreamMap::new();

    let weather_service = client
        .service_builder()
        .description("This is a great weather microservice")
        .queue_group("svc.weather")
        .start("weather", "0.1.0")
        .await?;

    let math_service = client
        .service_builder()
        .description("This is a great math microservice")
        .queue_group("svc.math")
        .start("math", "0.1.0")
        .await?;

    let add_endpoint = math_service.endpoint("svc.math.v1.add").await?;
    let add_handler = AddHandler::new();
    stream_map.insert("svc.math.v1.add", add_endpoint);

    let weather_endpoint = weather_service.endpoint("svc.weather.v1").await?;
    let weather_handler = WeatherHandler::new();
    stream_map.insert("svc.weather.v1", weather_endpoint);

    println!("Starting to listen on {}", address);
    while let Some((key, request)) = stream_map.next().await {
        match key {
            "svc.math.v1.add" => {
                add_handler.handle(request).await?;
            }
            "svc.weather.v1" => {
                weather_handler.handle(request).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
