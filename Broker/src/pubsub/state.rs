
use shared::game_sockets::{
    GameConnection, GameStream
};
use shared::protocol::broker::{ClientId, ShardId, Topic};
use std::collections::{
    HashMap, HashSet
};
use std::ptr::null;

#[derive(Default)]
pub struct PubSubState {
    pub topic_subscribers: HashMap<Topic, HashSet<ClientId>>,
    pub client_topics: HashMap<ClientId, HashSet<Topic>>,
    pub client_connections: HashMap<ClientId, GameConnection>,
    pub connection_clients: HashMap<GameConnection, ClientId>,
    pub shard_streams_by_topic: HashMap<Topic, (GameConnection, GameStream)>,
    pub spatial_service_streams: HashMap<GameConnection, GameStream>,
    next_client_id: ClientId,
}
impl PubSubState {
    pub fn allocate_client_id(&mut self) -> ClientId {
        if self.next_client_id.0 == 0 {
            self.next_client_id = ClientId(1);
        }

        let client_id = self.next_client_id;
        self.next_client_id = ClientId(self.next_client_id.0 + 1);

        tracing::info!("allocated client_id={}", client_id.0);

        client_id
    }

    pub fn register_client_connection(
        &mut self,
        client_id: ClientId,
        connection: GameConnection,
    ) {
        tracing::info!(
            "register client={} connection={}",
            client_id.0,
            connection.connection_id
        );

        if let Some(previous_connection) = self.client_connections.insert(client_id, connection) {
            if previous_connection != connection {
                tracing::warn!(
                    "client={} was already registered on connection {}; replacing with connection {}",
                    client_id.0,
                    previous_connection.connection_id,
                    connection.connection_id
                );

                self.connection_clients.remove(&previous_connection);
            }
        }

        self.connection_clients.insert(connection, client_id);
    }

    pub fn register_spatial_service(
        &mut self,
        connection: GameConnection,
        stream: GameStream,
    ) {
        tracing::info!(
            "register spatial service stream connection={} stream={}",
            connection.connection_id,
            stream.stream_id
        );

        self.spatial_service_streams.insert(connection, stream);
    }


    pub fn subscribe_registered_client(
        &mut self,
        client_id: ClientId,
        shard_id: ShardId,
    ) {

        let topic = Topic::ShardInstance(shard_id);

        tracing::info!(
            "subscribe registered client={} topic={}",
            client_id.0,
            &topic.to_string()
        );

        self.topic_subscribers
            .entry(topic)
            .or_default()
            .insert(client_id);

        self.client_topics
            .entry(client_id)
            .or_default()
            .insert(topic);
    }

    pub fn unsubscribe_client(
        &mut self,
        client_id: ClientId,
        shard_id: ShardId) {

        let topic = Topic::ShardInstance(shard_id);
        tracing::info!(
            "unsubscribe client={} topic={}",
            client_id.0,
            &topic.to_string()
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

    pub fn input_topic_for_client(
        &self,
        client_id: ClientId,
    ) -> Option<Topic> {
        //TODO get the player active shard
        let topic: Option<Topic> = Some(Topic::ShardInstance(ShardId(0)));
        return topic;
    }

    pub fn register_shard_topic(
        &mut self,
        shard_id: ShardId,
        connection: GameConnection,
        stream: GameStream,
    ) {
        let topic = Topic::ShardInstance(shard_id);
        tracing::debug!(
            "register shard stream for topic={} connection={} stream={}",
            &topic.to_string(),
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

        let removed_shard_topics: Vec<Topic> = self
            .shard_streams_by_topic
            .iter()
            .filter_map(|(topic, (shard_connection, _))| {
                if *shard_connection == connection {
                    Some(*topic)
                } else {
                    None
                }
            })
            .collect();

        self.shard_streams_by_topic
            .retain(|_, (shard_connection, _)| *shard_connection != connection);

        for topic in removed_shard_topics {
            self.remove_dead_shard_topic(topic);
        }
    }

    fn remove_dead_shard_topic(&mut self, topic: Topic) {
        tracing::warn!(
            "removing subscriptions and authorities for disconnected shard topic={}",
            &topic.to_string()
        );

        if let Some(clients) = self.topic_subscribers.remove(&topic) {
            for client_id in clients {
                if let Some(topics) = self.client_topics.get_mut(&client_id) {
                    topics.remove(&topic);

                    if topics.is_empty() {
                        self.client_topics.remove(&client_id);
                    }
                }

            }
        }

    }
}