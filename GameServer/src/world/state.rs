use std::collections::VecDeque;
use std::sync::Arc;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tokio::sync::{Mutex, MutexGuard};
use shared::protocol::{ClientId, EntityId, PlayerSnapshot};
use shared::protocol::snapshots::entity_snapshot::EntitySnapshot;

/**
World interaction should always be done through the SharedPlayerRegistry resource.
All interaction on entities must be done with entity_reg_shared,
client_reg_shared is used to get an entity_id from the client_id,
then we use the entity_id on entity_reg_shared to interact with the entity.
**/
#[derive(Resource, Clone)]
pub struct SharedEntityRegistry {
    pub entity_reg_shared: Arc<Mutex<EntityRegistry>>,
    pub client_reg_shared: Arc<Mutex<ClientEntityRegistry>>,
}


/**
When accessing the SharedEntityRegistry, use the try_lock method to acquire both locks.
This ensures that both locks are acquired atomically, preventing partial lock acquisition and potential deadlocks.

Easy copy and paste:

        match shared_registry.try_lock() {
            Some((cli_registry, ent_registry))=> {
                // Do Something
            }
            None => {
                tracing::warn!("could not lock player registry for client input");
                return;
            }
        }


**/
impl SharedEntityRegistry {
    pub fn try_lock(&'_ self) -> Option<(MutexGuard<'_, ClientEntityRegistry>, MutexGuard<'_, EntityRegistry>)> {
        let client_lock = self.client_reg_shared.try_lock().ok()?;
        let entity_lock = self.entity_reg_shared.try_lock().ok()?;
        Some((client_lock, entity_lock))
    }
}


#[derive(Resource, Default)]
pub struct EntityRegistry {
    pub by_network_id: HashMap<EntityId, Entity>,
    pub by_bevy_entity: HashMap<Entity, EntityId>,
}

impl EntityRegistry {
    pub fn insert(&mut self, entity_id: EntityId, bevy_entity: Entity) {
        self.by_network_id.insert(entity_id, bevy_entity);
        self.by_bevy_entity.insert(bevy_entity, entity_id);
    }

    pub fn remove_by_entity_id(&mut self, entity_id: &EntityId) -> Option<Entity> {
        let bevy_entity = self.by_network_id.remove(entity_id)?;
        self.by_bevy_entity.remove(&bevy_entity);
        Some(bevy_entity)
    }

    pub fn remove_by_bevy_entity(&mut self, bevy_entity: &Entity) -> Option<EntityId> {
        let entity_id = self.by_bevy_entity.remove(bevy_entity)?;
        self.by_network_id.remove(&entity_id);
        Some(entity_id)
    }

    pub fn get_bevy_entity(&self, entity_id: &EntityId) -> Option<Entity> {
        self.by_network_id.get(entity_id).copied()
    }
}

#[derive(Resource, Default)]
pub struct ClientEntityRegistry {
    pub client_to_entity: HashMap<ClientId, EntityId>,
    pub entity_to_client: HashMap<EntityId, ClientId>,
}

impl ClientEntityRegistry {
    pub fn insert(&mut self, client_id: ClientId, entity_id: EntityId) {
        self.client_to_entity.insert(client_id, entity_id);
        self.entity_to_client.insert(entity_id, client_id);
    }

    pub fn remove_client(&mut self, client_id: &ClientId) -> Option<EntityId> {
        let entity_id = self.client_to_entity.remove(client_id)?;
        self.entity_to_client.remove(&entity_id);
        Some(entity_id)
    }

    pub fn remove_entity(&mut self, entity_id: &EntityId) -> Option<ClientId> {
        let client_id = self.entity_to_client.remove(entity_id)?;
        self.client_to_entity.remove(&client_id);
        Some(client_id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EntityIdRange {
    pub next: u32,
    pub end_exclusive: u32,
}

#[derive(Resource, Debug, Default)]
pub struct EntityIdAllocator {
    pub ranges: VecDeque<EntityIdRange>,
    pub pending_request: bool,
}

impl EntityIdAllocator {
    pub fn allocate(&mut self) -> Option<EntityId> {
        let range = self.ranges.front_mut()?;

        if range.next >= range.end_exclusive {
            self.ranges.pop_front();
            return self.allocate();
        }

        let entity_id = EntityId(range.next);
        range.next += 1;

        if range.next >= range.end_exclusive {
            self.ranges.pop_front();
        }

        Some(entity_id)
    }

    pub fn add_range(&mut self, start: u32, count: u32) {
        if count == 0 {
            return;
        }

        let Some(end_exclusive) = start.checked_add(count) else {
            tracing::warn!(
                "invalid entity id range start={} count={}",
                start,
                count
            );
            return;
        };

        self.ranges.push_back(EntityIdRange {
            next: start,
            end_exclusive,
        });

        self.pending_request = false;
    }

    pub fn remaining(&self) -> u32 {
        self.ranges
            .iter()
            .map(|range| range.end_exclusive.saturating_sub(range.next))
            .sum()
    }
}