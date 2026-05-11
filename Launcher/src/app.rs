use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use crate::login::LoginPlugin;

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MMORPG Launcher".to_string(),
                resolution: (900, 600).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(LoginPlugin)
        .run();
}