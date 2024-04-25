use std::{sync::Arc, time::Duration};

extern crate anyhow;
extern crate reqwest;

use async_nats::service::Request;
use bytes::Bytes;
use nats_microservice_rs::Handler;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug)]
pub struct WeatherRequest(String);

impl TryFrom<Arc<Request>> for WeatherRequest {
    type Error = std::string::FromUtf8Error;

    fn try_from(value: Arc<Request>) -> Result<Self, Self::Error> {
        let string = String::from_utf8(value.message.payload.to_vec())?;
        Ok(Self(string))
    }
}

#[derive(Debug, Serialize)]
pub struct WeatherResponse {
    location: Location,
}

impl TryInto<Bytes> for WeatherResponse {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<Bytes, Self::Error> {
        let body = serde_json::to_vec(&self)?;
        Ok(body.into())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed geocode provided location")]
    FetchLocation(#[source] reqwest::Error),
}

#[derive(Debug, Deserialize, Serialize)]
struct Location {
    lat: f64,
    lon: f64,
}

pub struct WeatherHandler;

impl WeatherHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Handler for WeatherHandler {
    type Input = WeatherRequest;
    type Output = WeatherResponse;

    async fn compute(&self, user_input: Self::Input) -> Result<Self::Output, anyhow::Error> {
        println!("received: {:?}", user_input);

        let client = reqwest::Client::builder()
            .user_agent("curl/7.81.0")
            .build()
            .unwrap();

        let location: Location = client
            .get("https://nominatim.openstreetmap.org/search")
            .header("Accept", "application/json")
            .query(&[("q", user_input.0), ("format", String::from("json"))])
            .timeout(Duration::from_millis(500))
            .send()
            .await
            .map_err(Error::FetchLocation)?
            .error_for_status()
            .map_err(Error::FetchLocation)?
            .json()
            .await
            .map_err(Error::FetchLocation)?;

        // let weather = client
        //     .get("https://api.open-meteo.com/v1/forecast");

        Ok(WeatherResponse { location })
    }
}
