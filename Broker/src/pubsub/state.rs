
use shared::game_sockets::{
    GameConnection, GameStream
};
use shared::protocol::{ClientId, EntityId, ShardId, Topic, Username};
use std::collections::{
    HashMap, HashSet
};
use crate::net::peer_roles::PeerRole;


pub struct GhostRoute {
    pub from_shard_id: ShardId,
    pub to_shard_id: ShardId,
}

#[derive(Default)]
pub struct PubSubState {
    // Client
    next_client_id: ClientId,
    pub client_connections: HashMap<ClientId, (GameConnection, GameStream)>,
    pub client_username: HashMap<ClientId, Username>,
    pub topic_subscribers: HashMap<Topic, HashSet<ClientId>>,
    pub client_topics: HashMap<ClientId, HashSet<Topic>>,
    //entity
    pub entity_clients: HashMap<EntityId, Option<ClientId>>,
    pub client_entity: HashMap<ClientId, EntityId>,
    pub ghost_entity: HashMap<EntityId, GhostRoute>,
    //shard
    pub shard_streams_by_topic: HashMap<Topic, (GameConnection, GameStream)>,
    // spatial
    pub spatial_service_streams: Option<(GameConnection, GameStream)>,

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
        client_id: &ClientId,
        username: &Username,
        connection: &GameConnection,
        stream: &GameStream,
    ) {
        tracing::info!(
            "register client={} connection={}",
            client_id.0,
            connection.connection_id
        );

        if let Some(previous_connection) = self.client_connections.insert(client_id.clone(), (connection.clone(),stream.clone())) {
            if previous_connection.0 != *connection {
                tracing::warn!(
                    "client={} was already registered on connection {}; replacing with connection {}",
                    client_id.0,
                    previous_connection.0.connection_id,
                    connection.connection_id
                );

                self.client_connections.remove(&client_id);
            }
        }
        self.client_connections.insert(*client_id, (connection.clone(),stream.clone()));
        self.client_username.insert(client_id.clone(), username.clone());

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

        self.spatial_service_streams = Some((connection, stream));
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
        topic: Topic
    ) {

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
         topic
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

    pub fn remove_connection(
        &mut self,
        peer_role : PeerRole,
        connection: GameConnection,
        stream: GameStream)
    {

        match peer_role {
            PeerRole::Client => {
                let client_id_to_remove = self.client_connections
                    .iter()
                    .find(|(_, (conn, str))| *conn == connection && *str == stream)
                    .map(|(client_id, _)| *client_id);

                if client_id_to_remove.is_some() {
                    self.client_connections.remove(&client_id_to_remove.unwrap());

                    if let Some(topics) = self.client_topics.remove(&&client_id_to_remove.unwrap()) {
                        for topic in topics {
                            if let Some(subscribers) = self.topic_subscribers.get_mut(&topic) {
                                subscribers.remove(&&client_id_to_remove.unwrap());

                                if subscribers.is_empty() {
                                    self.topic_subscribers.remove(&topic);
                                }
                            }
                        }
                    }

                    if let Some(entity_id) = self.client_entity.remove(&client_id_to_remove.unwrap()) {
                        self.entity_clients.remove(&entity_id);
                    }
                }
            }
            PeerRole::Shard => {
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
            },
            _ => {}
        };


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

    pub fn get_shard_connection_and_stream(&mut self, shard_id: ShardId) -> Option<&(GameConnection, GameStream)> {

        let topic = Topic::ShardInstance(shard_id);
        let shard_connection = match self.shard_streams_by_topic.get(&topic) {
            Some(connection) => connection,
            None => {
                tracing::warn!("no shard connection found for topic: {:?}", topic);
                return None;
            }
        };

        Some(shard_connection)

    }
}