pub mod network_resources;
pub mod ui_resources;

use bevy::prelude::*;
use network_resources::NetworkingResourcesPlugin;
use ui_resources::UIResourcesPlugin;

pub struct ResourceLoaderPlugin;

impl Plugin for ResourceLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkingResourcesPlugin)
            .add_plugins(UIResourcesPlugin);
    }
}
