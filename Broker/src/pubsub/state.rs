use bimap::BiMap;
use shared::game_sockets::{
    GameConnection, GameStream
};
use shared::protocol::{ClientId, EntityId, ShardId, Topic, Username};
use std::collections::{
    HashMap, HashSet
};
use shared::protocol::net_handles::spatial_handler::SpatialHandle;
use crate::net::peer_roles::PeerRole;


pub struct GhostRoute {
    pub from_shard_id: ShardId,
    pub to_shard_id: ShardId,
}

#[derive(Hash, Clone)]
pub struct ConnectionStream{
    pub connection: GameConnection,
    pub stream: GameStream,
}

impl Eq for ConnectionStream{}
impl PartialEq for ConnectionStream {
    fn eq(&self, other: &Self) -> bool {
       self.connection == other.connection && self.stream == other.stream
    }
}


#[derive(Default)]
pub struct PubSubState {
    // Client
    next_client_id: ClientId,
    pub client_connections: BiMap<ClientId, ConnectionStream>,
    pub client_username: HashMap<ClientId, Username>,

    pub topic_subscribers: HashMap<Topic, HashSet<ClientId>>,
    pub client_topics: HashMap<ClientId, HashSet<Topic>>,
    //entity
    pub ghost_entity: HashMap<EntityId, GhostRoute>,
    //shard
    pub shard_streams_by_topic: BiMap<Topic, ConnectionStream>,
    // spatial
    pub spatial_handle: SpatialHandle,

}
impl PubSubState {
    pub fn allocate_client_id(
        &mut self
    ) -> ClientId {
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

        let connection_stream = ConnectionStream{
            connection: connection.clone(),
            stream : stream.clone()
        };


        self.client_connections.insert(*client_id, connection_stream);
        self.client_username.insert(client_id.clone(), username.clone());

    }

    pub fn register_spatial_service(
        &mut self,
        connection: &GameConnection,
        stream: &GameStream,
    ) {
        tracing::info!(
            "register spatial service stream connection={} stream={}",
            connection.connection_id,
            stream.stream_id
        );

        self.spatial_handle.connection = Some(connection.clone());
        self.spatial_handle.stream = Some(stream.clone());

    }


    pub fn subscribe_registered_client(
        &mut self,
        client_id: ClientId,
        topic: Topic,
    ) {
        

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
        self.client_topics
            .get(&client_id)?
            .iter()
            .find(|topic| matches!(topic, Topic::ShardInstance { .. }))
            .copied()
    }

    pub fn register_shard_topic(
        &mut self,
        shard_id: ShardId,
        connection: GameConnection,
        stream: GameStream,
    ) {
        let topic = Topic::ShardInstance{
            id : shard_id
        };
        tracing::debug!(
            "register shard stream for topic={} connection={} stream={}",
            &topic.to_string(),
            connection.connection_id,
            stream.stream_id
        );

        let connection_stream = ConnectionStream{
            connection: connection.clone(),
            stream : stream.clone()
        };


        self.shard_streams_by_topic
            .insert(topic, connection_stream);
    }

    pub fn remove_connection(
        &mut self,
        peer_role : PeerRole,
        connection: GameConnection,
        stream: GameStream
    ) {

        let connection_stream = ConnectionStream{
            connection: connection.clone(),
            stream : stream.clone()
        };

        match peer_role {
            PeerRole::Client => {
                match self.client_connections.remove_by_right(&connection_stream){
                    Some((client_id_to_remove,..)) => {

                        if let Some(topics) = self.client_topics.remove(&client_id_to_remove) {
                            for topic in topics {
                                if let Some(subscribers) = self.topic_subscribers.get_mut(&topic) {
                                    subscribers.remove(&client_id_to_remove);

                                    if subscribers.is_empty() {
                                        self.topic_subscribers.remove(&topic);
                                    }
                                }
                            }
                        }

                        self.client_username.remove(&client_id_to_remove);

                        tracing::debug!(
                            "removed connection={} stream={}",
                            connection.connection_id,
                            stream.stream_id
                        );
                    }
                    None => {
                        tracing::warn!(
                            "could not remove connection={} stream={}",
                            connection.connection_id,
                            stream.stream_id
                        );
                    }
                }
            }

            PeerRole::Shard => {
                let removed_shard_topics: Vec<Topic> = self
                    .shard_streams_by_topic
                    .iter()
                    .filter_map(|(topic, connection_stream)| {
                        if connection_stream.connection == connection {
                            Some(*topic)
                        } else {
                            None
                        }
                    })
                    .collect();

                self.shard_streams_by_topic
                    .retain(|_, connection_stream| connection_stream.connection == connection);

                for topic in removed_shard_topics {
                    self.remove_dead_shard_topic(topic);
                }
            },
            PeerRole::SpatialService =>{
                self.spatial_handle.connection = None;
                self.spatial_handle.stream = None;
            }
        };
    }

    fn remove_dead_shard_topic(
        &mut self,
        topic: Topic
    ) {
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

    pub fn get_connection_stream_by_shard(
        &mut self,
        topic: Topic
    ) -> Option<&ConnectionStream> {

        let shard_connection = match self.shard_streams_by_topic.get_by_left(&topic) {
            Some(connection) => connection,
            None => {
                tracing::warn!("no shard connection found for topic: {:?}", topic);
                return None;
            }
        };

        Some(shard_connection)
    }


    pub fn get_shard_by_connection_stream(
        &mut self,
        connection_stream: &ConnectionStream
    ) -> Option<&Topic> {

        let shard_topic = match self.shard_streams_by_topic.get_by_right(&connection_stream) {
            Some(topic) => topic,
            None => {
                tracing::warn!("no shard topic found for connection: {:?}", connection_stream.connection.connection_id);
                return None;
            }
        };

        Some(shard_topic)
    }

    pub fn get_client_id_by_connection_stream(
        &self,
        connection_stream: &ConnectionStream
    ) -> Option<ClientId> {
        let client_id = match self.client_connections.get_by_right(connection_stream){
            Some(client_id) => *client_id,
            None => return None,
        };

        Some(client_id)
    }


    pub fn get_connection_stream_by_client_id(
        &self,
        client_id: &ClientId
    ) -> Option<&ConnectionStream> {
        let connection_stream = match self.client_connections.get_by_left(client_id){
            Some(connection) => connection,
            None => return None,
        };

        Some(&connection_stream)
    }

}