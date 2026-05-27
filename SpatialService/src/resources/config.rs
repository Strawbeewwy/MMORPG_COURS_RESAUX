use bevy::prelude::*;
use std::env;
use shared::config::{
    DEFAULT_BROKER_HOST, DEFAULT_BROKER_PORT, DEFAULT_CROSSING_MARGIN,
    DEFAULT_QUAD_TREE_MAX_DEPTH, DEFAULT_SPATIAL_HOST, DEFAULT_SPATIAL_LISTEN_PORT,
    DEFAULT_WORLD_HALF_SIZE,
};

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone, Resource)]
pub struct SpatialConfig {
    /// Address the spatial service listens on for incoming shard connections.
    pub listen_host: String,
    pub listen_port: u16,
    /// Address of the broker the spatial service connects to.
    pub broker_host: String,
    pub broker_port: u16,
    /// World half-extent used to build the QuadTree root bounds.
    pub world_half_size: f32,
    /// Max subdivision depth of the QuadTree.
    pub quad_tree_max_depth: u8,
    /// Radius (world units) that triggers a CrossingAlert.
    pub crossing_margin: f32,
}

impl SpatialConfig {
    pub fn from_env() -> Self {
        Self {
            listen_host: env::var("SPATIAL_HOST")
                .unwrap_or_else(|_| DEFAULT_SPATIAL_HOST.to_string()),
            listen_port: env::var("SPATIAL_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SPATIAL_LISTEN_PORT),
            broker_host: env::var("BROKER_HOST")
                .unwrap_or_else(|_| DEFAULT_BROKER_HOST.to_string()),
            broker_port: env::var("BROKER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_BROKER_PORT),
            world_half_size: env::var("WORLD_HALF_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_WORLD_HALF_SIZE),
            quad_tree_max_depth: env::var("QUAD_TREE_MAX_DEPTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_QUAD_TREE_MAX_DEPTH),
            crossing_margin: env::var("CROSSING_MARGIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_CROSSING_MARGIN),
        }
    }
}

