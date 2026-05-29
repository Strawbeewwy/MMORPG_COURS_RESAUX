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

        BrokerMessage::Publish { shard_id, payload } => {
            encode_publish(Topic::ShardInstance(*shard_id), payload)
        }

        BrokerMessage::Broadcast { payload } => {
            encode_broadcast(payload)
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
    }
}

fn encode_position_update(client_id: ClientId, positions: &[f32; 2]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + 2 * size_of::<f32>());


    let id: u32 = client_id.into();
    packet.push(TAG_POSITION_UPDATE);
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&positions[0].to_le_bytes());
    packet.extend_from_slice(&positions[1].to_le_bytes());

    packet
}

fn encode_register_shard(topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    packet.push(TAG_REGISTER_SHARD);
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_register_spatial_service() -> Vec<u8> {
    vec![TAG_REGISTER_SPATIAL_SERVICE]
}

fn encode_subscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    let id: u32 = client_id.into();
    packet.push(TAG_SUBSCRIBE);
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}


fn encode_unsubscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    let id: u32 = client_id.into();
    packet.push(TAG_UNSUBSCRIBE);
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_publish(topic: Topic, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN + MAX_PAYLOAD_LEN + payload.len());

    packet.push(TAG_PUBLISH);
    packet.extend_from_slice(&topic.to_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_broadcast(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + MAX_PAYLOAD_LEN + payload.len());

    packet.push(TAG_BROADCAST);
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
    packet.push(TAG_CLIENT_INPUT);
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

    packet.push(TAG_CLIENT_HELLO);
    packet.extend_from_slice(&(username_bytes.len() as u16).to_be_bytes());
    packet.extend_from_slice(username_bytes);

    packet
}

fn encode_client_accepted(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    packet.push(TAG_CLIENT_ACCEPTED);
    packet.extend_from_slice(&client_id.0.to_le_bytes());

    packet
}