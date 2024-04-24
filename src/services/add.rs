use async_nats::service::Request;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AddRequest {
    a: f64,
    b: f64,
}

impl TryFrom<&Request> for AddRequest {
    type Error = serde_json::Error;

    fn try_from(value: &Request) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value.message.payload)
    }
}

#[derive(Debug, Serialize)]
pub struct AddResponse {
    result: f64,
}

impl TryInto<Bytes> for AddResponse {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<Bytes, Self::Error> {
        let body = serde_json::to_vec(&self)?;
        Ok(body.into())
    }
}

pub fn add(req: AddRequest) -> Result<AddResponse, anyhow::Error> {
    Ok(AddResponse {
        result: req.a + req.b,
    })
}
