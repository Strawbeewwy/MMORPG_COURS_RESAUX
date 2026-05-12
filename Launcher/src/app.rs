/**
This file is the entry point of the bevy app.
It initializes the app and adds all plugins.
We use the bevy_egui plugin to add a GUI.
The LoginPlugin is responsible for the systems authentication
with the gatekeeper server.
We also use the default plugins, but we could selectively
disable some of them since we just need a GUI.
**/


use bevy::prelude::*;
use crate::systems::SystemLoaderPlugin;
use crate::resources::ResourceLoaderPlugin;

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
        .add_systems(Startup, setup_camera)
        .add_plugins(SystemLoaderPlugin)
        .add_plugins(ResourceLoaderPlugin)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}