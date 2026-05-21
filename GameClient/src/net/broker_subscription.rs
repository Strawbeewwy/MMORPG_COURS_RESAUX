use crate::net::broker_client::BrokerClient;
use shared::protocol::broker::{
    Topic, encode_subscribe, encode_unsubscribe, topic_to_string,
};

pub fn subscribe_to_configured_topics(
    broker_client: &mut BrokerClient,
    client_id: u32,
    topics: &[Topic],
) {
    for topic in topics {
        subscribe_to_topic(broker_client, client_id, *topic);
    }
}

pub fn subscribe_to_topic(
    broker_client: &mut BrokerClient,
    client_id: u32,
    topic: Topic,
) {
    if broker_client.subscribed_topics.contains(&topic) {
        tracing::debug!(
            "client {} already subscribed to topic {}",
            client_id,
            topic_to_string(&topic)
        );
        return;
    }

    let packet = encode_subscribe(client_id, topic);

    if !broker_client.send_raw(packet) {
        tracing::error!(
            "failed to subscribe client {} to topic {}",
            client_id,
            topic_to_string(&topic)
        );
        return;
    }

    broker_client.subscribed_topics.insert(topic);

    tracing::info!(
        "subscribed client {} to broker topic {}",
        client_id,
        topic_to_string(&topic)
    );
}

pub fn unsubscribe_from_topic(
    broker_client: &mut BrokerClient,
    client_id: u32,
    topic: Topic,
) {
    if !broker_client.subscribed_topics.contains(&topic) {
        tracing::debug!(
            "client {} is not subscribed to topic {}",
            client_id,
            topic_to_string(&topic)
        );
        return;
    }

    let packet = encode_unsubscribe(client_id, topic);

    if !broker_client.send_raw(packet) {
        tracing::error!(
            "failed to unsubscribe client {} from topic {}",
            client_id,
            topic_to_string(&topic)
        );
        return;
    }

    broker_client.subscribed_topics.remove(&topic);

    tracing::info!(
        "unsubscribed client {} from broker topic {}",
        client_id,
        topic_to_string(&topic)
    );
}