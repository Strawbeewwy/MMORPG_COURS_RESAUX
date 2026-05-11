/**
login.rs is the root module for the login authentication.
It imports all other modules that are needed for the login.
login/network.rs
login/resources.rs
login/systems.rs
**/



pub mod network;
mod systems;
mod resources;

use bevy::prelude::*;

use resources::LoginResourcesPlugin;
use systems::LoginSystemsPlugin;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoginResourcesPlugin)
            .add_plugins(LoginSystemsPlugin);
    }
}