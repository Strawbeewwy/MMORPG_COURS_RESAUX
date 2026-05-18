pub mod login_system;
pub mod ui_system;
pub mod launch_game_system;

use bevy::prelude::*;
use login_system::LoginSystemPlugin;
use ui_system::UISystemPlugin;
use crate::systems::launch_game_system::LaunchGameSystemPlugin;

pub struct SystemLoaderPlugin;

impl Plugin for SystemLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoginSystemPlugin)
            .add_plugins(UISystemPlugin)
            .add_plugins(LaunchGameSystemPlugin);
    }
}
