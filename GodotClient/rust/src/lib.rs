//! GDExtension entry point for the MMO client.
//!
//! This crate is compiled as a `cdylib` and loaded by Godot 4 at runtime
//! via the `mmo_client.gdextension` manifest.
//!
//! Module layout:
//!   network/ — async QUIC/TCP client that speaks the shared broker protocol
//!   world/   — entity registry, position interpolation, state tracking
//!   ui/      — Godot Node bindings for HUD elements
use godot::prelude::*;

mod network;
mod ui;
mod world;

/// Extension registration — Godot calls `gdext_rust_init` generated here.
struct MmoClientExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MmoClientExtension {}

