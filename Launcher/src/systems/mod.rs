mod login_system;
mod ui_system;

use bevy::prelude::*;
use login_system::LoginSystemPlugin;
use ui_system::UISystemPlugin;

pub struct SystemLoaderPlugin;

impl Plugin for SystemLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoginSystemPlugin)
            .add_plugins(UISystemPlugin);
    }
}
