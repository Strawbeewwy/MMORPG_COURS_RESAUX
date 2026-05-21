use bevy::prelude::*;
use shared::game_sockets::{
    GameConnection, GameStream
};
use shared::protocol::broker::{
    ClientId, Topic, topic_to_string
};
use std::collections::{
    HashMap, HashSet
};

#[derive(Resource, Default)]
pub struct PubSubState {
    pub topic_subscribers: HashMap<Topic, HashSet<ClientId>>,
    pub client_topics: HashMap<ClientId, HashSet<Topic>>,
    pub client_connections: HashMap<ClientId, GameConnection>,
    pub connection_clients: HashMap<GameConnection, ClientId>,
    pub shard_streams_by_topic: HashMap<Topic, (GameConnection, GameStream)>,
}
impl PubSubState {
    pub fn subscribe_client(
        &mut self,
        client_id: ClientId,
        topic: Topic,
        connection: GameConnection,
    ) {
        tracing::info!(
            "subscribe client={} topic={}",
            client_id,
            topic_to_string(&topic)
        );

        self.client_connections.insert(client_id, connection);
        self.connection_clients.insert(connection, client_id);

        self.topic_subscribers
            .entry(topic)
            .or_default()
            .insert(client_id);

        self.client_topics
            .entry(client_id)
            .or_default()
            .insert(topic);
    }

    pub fn unsubscribe_client(&mut self, client_id: ClientId, topic: Topic) {
        tracing::info!(
            "unsubscribe client={} topic={}",
            client_id,
            topic_to_string(&topic)
        );

        if let Some(subscribers) = self.topic_subscribers.get_mut(&topic) {
            subscribers.remove(&client_id);

            if subscribers.is_empty() {
                self.topic_subscribers.remove(&topic);
            }
        }

        if let Some(topics) = self.client_topics.get_mut(&client_id) {
            topics.remove(&topic);

            if topics.is_empty() {
                self.client_topics.remove(&client_id);
            }
        }
    }

    pub fn register_client_connection(
        &mut self,
        client_id: ClientId,
        connection: GameConnection,
    ) {
        self.client_connections.insert(client_id, connection);
        self.connection_clients.insert(connection, client_id);
    }

    pub fn register_shard_topic(
        &mut self,
        topic: Topic,
        connection: GameConnection,
        stream: GameStream,
    ) {
        tracing::debug!(
            "register shard stream for topic={} connection={} stream={}",
            topic_to_string(&topic),
            connection.connection_id,
            stream.stream_id
        );

        self.shard_streams_by_topic
            .insert(topic, (connection, stream));
    }

    pub fn remove_connection(&mut self, connection: GameConnection) {
        if let Some(client_id) = self.connection_clients.remove(&connection) {
            self.client_connections.remove(&client_id);

            if let Some(topics) = self.client_topics.remove(&client_id) {
                for topic in topics {
                    if let Some(subscribers) = self.topic_subscribers.get_mut(&topic) {
                        subscribers.remove(&client_id);

                        if subscribers.is_empty() {
                            self.topic_subscribers.remove(&topic);
                        }
                    }
                }
            }
        }

        self.shard_streams_by_topic
            .retain(|_, (shard_connection, _)| *shard_connection != connection);
    }

    pub fn first_topic_for_client(&self, client_id: ClientId) -> Option<Topic> {
        self.client_topics
            .get(&client_id)
            .and_then(|topics| topics.iter().next().copied())
    }
}