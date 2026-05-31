use crate::protocol::broker::config::*;

pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, ClientId, BrokerMessage,
};

pub use crate::protocol::broker::topic::{
    TOPIC_LEN, ShardId,Topic,
};


pub fn encode_message(message: &BrokerMessage) -> anyhow::Result<Vec<u8>> {
    match message {
        BrokerMessage::Subscribe { client_id, shard_id } => {
            Ok(encode_subscribe(*client_id, Topic::ShardInstance(*shard_id)))
        }

        BrokerMessage::Unsubscribe { client_id, shard_id } => {
            Ok(encode_unsubscribe(*client_id, Topic::ShardInstance(*shard_id)))
        }

        BrokerMessage::Publish { shard_id,payload_len, payload } => {
            encode_publish(Topic::ShardInstance(*shard_id),*payload_len, payload)
        }

        BrokerMessage::Broadcast { payload_len, payload } => {
            encode_broadcast(*payload_len,payload)
        }

        BrokerMessage::ClientInput { client_id, input } => {
            Ok(encode_client_input(*client_id, *input))
        }

        BrokerMessage::RegisterShard { shard_id } => {
            Ok(encode_register_shard(Topic::ShardInstance(*shard_id)))
        }

        BrokerMessage::RegisterSpatialService => {
            Ok(encode_register_spatial_service())
        }

        BrokerMessage::ClientHello{username} => {
            Ok(encode_client_hello(username.clone()))
        }

        BrokerMessage::ClientAccepted { client_id } => {
            Ok(encode_client_accepted(*client_id))
        },
        BrokerMessage::PositionUpdate {
            client_id,
            position,
        } => {
            Ok(encode_position_update(*client_id, position))
        },

        BrokerMessage::ShardRegister { shard_id } => {
            Ok(encode_shard_register(*shard_id))
        },

        BrokerMessage::HandoffRequest { client_id, from_shard, to_shard } => {
            Ok(encode_handoff_request(*client_id, *from_shard, *to_shard))
        },

        BrokerMessage::HandoffAck { client_id, to_shard } => {
            Ok(encode_handoff_ack(*client_id, *to_shard))
        },
    }
}

fn encode_position_update(client_id: ClientId, positions: &[f32; 2]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + 2 * size_of::<f32>());

    let id: u32 = client_id.into();
    let tag= TAG_POSITION_UPDATE;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&positions[0].to_le_bytes());
    packet.extend_from_slice(&positions[1].to_le_bytes());

    packet
}

fn encode_register_shard(topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    let tag = TAG_REGISTER_SHARD;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_register_spatial_service() -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);
    
    let tag: u8 = TAG_REGISTER_SPATIAL_SERVICE;
    packet.extend_from_slice(&tag.to_le_bytes());

    packet
}

fn encode_subscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    let id: u32 = client_id.into();
    let tag: u8 = TAG_SUBSCRIBE;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}


fn encode_unsubscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    let id: u32 = client_id.into();
    let tag: u8 = TAG_UNSUBSCRIBE;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_publish(topic: Topic,payload_len: u16, payload: &[u8]) -> anyhow::Result<Vec<u8>> {

    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN + MAX_PAYLOAD_LEN + payload.len());

    let tag: u8 = TAG_PUBLISH;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_broadcast(payload_len:u16, payload: &[u8]) -> anyhow::Result<Vec<u8>> {

    let mut packet = Vec::with_capacity(TAG_LEN + MAX_PAYLOAD_LEN + payload.len());

    let tag: u8 = TAG_BROADCAST;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + CLIENT_INPUT_LEN);

    let id: u32 = client_id.into();
    let tag: u8 = TAG_CLIENT_INPUT;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&input);

    packet
}
fn encode_client_hello(
    username: String,
) -> Vec<u8> {
    let username_bytes = username.as_bytes();

    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username_bytes.len()
    );

    let tag: u8 = TAG_CLIENT_HELLO;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&(username_bytes.len() as u16).to_be_bytes());
    packet.extend_from_slice(username_bytes);

    packet
}

fn encode_client_accepted(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    let tag: u8 = TAG_CLIENT_ACCEPTED;
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&client_id.0.to_le_bytes());

    packet
}

fn encode_shard_register(shard_id: ShardId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + size_of::<u32>());
    packet.push(TAG_SHARD_REGISTER);
    packet.extend_from_slice(&shard_id.0.to_le_bytes());
    packet
}

fn encode_handoff_request(client_id: ClientId, from_shard: ShardId, to_shard: ShardId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + 2 * size_of::<u32>());
    packet.push(TAG_HANDOFF_REQUEST);
    packet.extend_from_slice(&client_id.0.to_le_bytes());
    packet.extend_from_slice(&from_shard.0.to_le_bytes());
    packet.extend_from_slice(&to_shard.0.to_le_bytes());
    packet
}

fn encode_handoff_ack(client_id: ClientId, to_shard: ShardId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + size_of::<u32>());
    packet.push(TAG_HANDOFF_ACK);
    packet.extend_from_slice(&client_id.0.to_le_bytes());
    packet.extend_from_slice(&to_shard.0.to_le_bytes());
    packet
}
