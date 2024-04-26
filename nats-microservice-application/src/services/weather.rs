use std::sync::Arc;

extern crate anyhow;
extern crate reqwest;

use async_nats::service::Request;
use bytes::Bytes;
use nats_microservice_rs::Handler;
use serde::{Deserialize, Deserializer, Serialize};
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
    temperature: String,
    relative_humidity: String,
    precipitation: String,
    cloud_cover: String,
    surface_pressure: String,
    wind_speed: String,
    wind_direction: String,
}

impl From<OpenMeteoResponse> for WeatherResponse {
    fn from(value: OpenMeteoResponse) -> Self {
        Self {
            location: Location {
                lat: value.latitude,
                lon: value.longitude,
                elevation: Some(value.elevation),
            },
            temperature: format!("{}{}", value.values.temperature, value.units.temperature),
            relative_humidity: format!(
                "{}{}",
                value.values.relative_humidity, value.units.relative_humidity
            ),
            precipitation: format!(
                "{}{}",
                value.values.precipitation, value.units.precipitation
            ),
            cloud_cover: format!("{}{}", value.values.cloud_cover, value.units.cloud_cover),
            surface_pressure: format!(
                "{}{}",
                value.values.surface_pressure, value.units.surface_pressure
            ),
            wind_speed: format!("{}{}", value.values.wind_speed, value.units.wind_speed),
            wind_direction: format!(
                "{}{}",
                value.values.wind_direction, value.units.wind_direction
            ),
        }
    }
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
    #[error("failed to geocode provided location: {0}")]
    FetchLocation(#[source] reqwest::Error),
    #[error("failed to fetch weather information: {0}")]
    FetchWeather(#[source] reqwest::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    #[serde(deserialize_with = "deserialize_float_string")]
    pub lat: f64,
    #[serde(deserialize_with = "deserialize_float_string")]
    pub lon: f64,
    #[serde(skip_deserializing)]
    pub elevation: Option<f64>,
}

fn deserialize_float_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoUnits {
    #[serde(rename = "temperature_2m")]
    temperature: String,
    #[serde(rename = "relative_humidity_2m")]
    relative_humidity: String,
    precipitation: String,
    cloud_cover: String,
    surface_pressure: String,
    #[serde(rename = "wind_speed_10m")]
    wind_speed: String,
    #[serde(rename = "wind_direction_10m")]
    wind_direction: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoValues {
    #[serde(rename = "temperature_2m")]
    temperature: f64,
    #[serde(rename = "relative_humidity_2m")]
    relative_humidity: f64,
    precipitation: f64,
    cloud_cover: f64,
    surface_pressure: f64,
    #[serde(rename = "wind_speed_10m")]
    wind_speed: f64,
    #[serde(rename = "wind_direction_10m")]
    wind_direction: f64,
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoResponse {
    latitude: f64,
    longitude: f64,
    elevation: f64,

    #[serde(rename = "current_units")]
    units: OpenMeteoUnits,
    #[serde(rename = "current")]
    values: OpenMeteoValues,
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
        let client = reqwest::Client::builder()
            .user_agent("curl/7.81.0")
            .build()
            .unwrap();

        let locations: Vec<Location> = client
            .get("https://nominatim.openstreetmap.org/search")
            .header("Accept", "application/json")
            .query(&[("q", user_input.0), ("format", String::from("json"))])
            .send()
            .await
            .map_err(Error::FetchLocation)?
            .error_for_status()
            .map_err(Error::FetchLocation)?
            .json()
            .await
            .map_err(Error::FetchLocation)?;

        let location = locations.first().unwrap().clone();

        let weather: OpenMeteoResponse = client
            .get("https://api.open-meteo.com/v1/forecast")
            .query(&[
                ("latitude", location.lat.to_string()),
                ("longitude", location.lon.to_string()),
                ("current", String::from("temperature_2m,relative_humidity_2m,precipitation,cloud_cover,surface_pressure,wind_speed_10m,wind_direction_10m")),
            ])
            .send()
            .await
            .inspect_err(|err| println!("{:?}", err))
            .map_err(Error::FetchWeather)?
            .error_for_status()
            .inspect_err(|err| println!("{:?}", err))
            .map_err(Error::FetchWeather)?
            .json()
            .await
            .inspect_err(|err| println!("{:?}", err))
            .map_err(Error::FetchWeather)?;

        Ok(WeatherResponse::from(weather))
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn serialize_location() {
        let res = json!([
          {
            "place_id": 130161328,
            "licence": "Data © OpenStreetMap contributors, ODbL 1.0. http://osm.org/copyright",
            "osm_type": "relation",
            "osm_id": 62705,
            "lat": "53.5574458",
            "lon": "13.2602781",
            "class": "boundary",
            "type": "administrative",
            "place_rank": 16,
            "importance": 0.5333234118727799,
            "addresstype": "town",
            "name": "Neubrandenburg",
            "display_name": "Neubrandenburg, Mecklenburgische Seenplatte, Mecklenburg-Vorpommern, Deutschland",
            "boundingbox": [
              "53.4441088",
              "53.6189803",
              "13.1367108",
              "13.3441823"
            ]
          }
        ]);

        let out: Vec<Location> = serde_json::from_value(res).unwrap();

        assert_eq!(1, out.len());
        assert_eq!(53.5574458, out[0].lat);
        assert_eq!(13.2602781, out[0].lon);
    }

    #[test]
    fn serialize_weather_response() {
        let res = json!({
          "latitude": 52.52,
          "longitude": 13.419998,
          "generationtime_ms": 0.05805492401123047,
          "utc_offset_seconds": 0,
          "timezone": "GMT",
          "timezone_abbreviation": "GMT",
          "elevation": 38,
          "current_units": {
            "time": "iso8601",
            "interval": "seconds",
            "temperature_2m": "°C",
            "relative_humidity_2m": "%",
            "precipitation": "mm",
            "cloud_cover": "%",
            "surface_pressure": "hPa",
            "wind_speed_10m": "km/h",
            "wind_direction_10m": "°"
          },
          "current": {
            "time": "2024-04-26T14:15",
            "interval": 900,
            "temperature_2m": 14.2,
            "relative_humidity_2m": 34,
            "precipitation": 0,
            "cloud_cover": 100,
            "surface_pressure": 1002.2,
            "wind_speed_10m": 14.9,
            "wind_direction_10m": 187
          }
        });

        let out: OpenMeteoResponse = serde_json::from_value(res).unwrap();

        assert_eq!(52.52, out.latitude);
        assert_eq!(13.419998, out.longitude);
    }
}
