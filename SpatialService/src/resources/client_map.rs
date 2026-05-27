use bevy::prelude::*;
use std::collections::HashMap;

/// Tracks each client's currently subscribed shard id.
/// Updated by the subscription system on every shard change.
#[derive(Resource, Default, Debug)]
pub struct ClientMap(pub HashMap<u32, u32>); // client_id → current shard_id

