/**
This file is the entry point of the bevy app.
It initializes the app and adds all plugins.
We use the bevy_egui plugin to add a GUI.
The LoginPlugin is responsible for the login authentication
with the gatekeeper server.
We also use the default plugins, but we could selectively
disable some of them since we just need a GUI.
**/


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