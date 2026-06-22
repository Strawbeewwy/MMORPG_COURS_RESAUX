
pub const WINDOW_TITLE: &str = "MMORPG Launcher";
pub const WINDOW_WIDTH: u32 = 900;
pub const WINDOW_HEIGHT: u32 = 600;

// Legacy GameClient binary (unused in this MMO architecture)
pub const DEFAULT_GC_BINARY: &str = "gameclient";

// Godot Client Configuration
// Path to the Godot executable relative to workspace root
#[cfg(windows)]
pub const GODOT_CLIENT_EXECUTABLE: &str = "GodotClient/.godot_bin/Godot_v4.6-stable_win64.exe";

#[cfg(not(windows))]
pub const GODOT_CLIENT_EXECUTABLE: &str = "GodotClient/game/.godot_bin/godot";

// Project file for Godot client
pub const GODOT_PROJECT_FILE: &str = "GodotClient/game/project.godot";

// Default Broker connection settings
pub const DEFAULT_BROKER_HOST: &str = "127.0.0.1";
pub const DEFAULT_BROKER_PORT: u16 = 9600;
