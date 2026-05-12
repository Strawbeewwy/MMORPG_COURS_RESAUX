pub mod network_resources;

use bevy::prelude::*;
use network_resources::NetworkingResources;

pub struct ResourceLoaderPlugin;
impl Plugin for ResourceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkingResources);
    }
}