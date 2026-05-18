use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

/*
helper functions for encoding and decoding messages.
can be changed later to encode binary instead of Json
*/
pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(message).context("failed to encode protocol message")
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).context("failed to decode protocol message")
}
