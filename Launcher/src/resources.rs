pub mod network_resources;
pub mod ui_resources;

use bevy::prelude::*;
use network_resources::NetworkingResources;
use ui_resources::UIResources;

pub struct ResourceLoaderPlugin;
impl Plugin for ResourceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkingResources)
            .add_plugins(UIResources);
    }
}