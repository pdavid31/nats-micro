use std::sync::Arc;

use async_nats::service::Request;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::handler::Handler;

#[derive(Debug, Deserialize)]
pub struct AddRequest {
    a: f64,
    b: f64,
}

impl TryFrom<Arc<Request>> for AddRequest {
    type Error = serde_json::Error;

    fn try_from(value: Arc<Request>) -> Result<Self, Self::Error> {
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

pub struct AddHandler;

impl AddHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler for AddHandler {
    type Input = AddRequest;
    type Output = AddResponse;

    async fn compute(&self, user_input: Self::Input) -> Result<Self::Output, anyhow::Error> {
        Ok(AddResponse {
            result: user_input.a + user_input.b,
        })
    }
}
