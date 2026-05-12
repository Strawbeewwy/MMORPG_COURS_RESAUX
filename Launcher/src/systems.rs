/**
systems is the root module for the systems authentication.
It imports all other modules that are needed for the systems.
systems/network.rs
systems/login_system
**/

pub mod network;
mod login_system;
mod ui_system;

use bevy::prelude::*;

use login_system::LoginSystemPlugin;
use crate::systems::ui_system::UISystemPlugin;

pub struct SystemLoaderPlugin;

impl Plugin for SystemLoaderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LoginSystemPlugin)
            .add_plugins(UISystemPlugin);
    }
}