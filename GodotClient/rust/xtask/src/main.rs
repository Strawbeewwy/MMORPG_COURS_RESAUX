//! Build-system automation for the GodotClient.
//!
//! Usage (run from the GodotClient/ or GodotClient/rust/ directory):
//!   cargo xtask setup    — download Godot 4.6 + export templates
//!   cargo xtask editor   — compile GDExtension + open the Godot editor
//!   cargo xtask run      — compile GDExtension + launch the game headlessly
//!   cargo xtask package  — release build + export standalone binary
//!
//! Godot binaries are stored in GodotClient/.godot_bin/ (gitignored).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ─── Version pin ─────────────────────────────────────────────────────────────
const GODOT_VERSION: &str = "4.6";
const GODOT_TAG: &str = "4.6-stable";

// ─── Entry point ─────────────────────────────────────────────────────────────
fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let workspace_root = workspace_root();

    match task {
        "setup" => setup(&workspace_root),
        "editor" => {
            build_gdextension(&workspace_root, false);
            copy_lib(&workspace_root);
            write_gdextension_file(&workspace_root);
            open_editor(&workspace_root);
        }
        "run" => {
            build_gdextension(&workspace_root, false);
            copy_lib(&workspace_root);
            write_gdextension_file(&workspace_root);
            run_game(&workspace_root);
        }
        "package" => {
            build_gdextension(&workspace_root, true);
            copy_lib(&workspace_root);
            write_gdextension_file(&workspace_root);
            export_game(&workspace_root);
        }
        _ => print_help(),
    }
}

// ─── Helpers — paths ─────────────────────────────────────────────────────────

/// Returns the GodotClient/ root (one level above rust/).
fn workspace_root() -> PathBuf {
    // xtask binary lives at GodotClient/rust/target/…/xtask(.exe)
    // We resolve relative to the cargo manifest dir set by cargo at build time.
    let manifest = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap());

    // manifest = GodotClient/rust/xtask  →  parent = rust  →  parent = GodotClient
    manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(&manifest)
        .to_path_buf()
}

fn godot_bin_dir(root: &Path) -> PathBuf {
    root.join(".godot_bin")
}

fn godot_exe(root: &Path) -> PathBuf {
    let dir = godot_bin_dir(root);
    if cfg!(target_os = "windows") {
        dir.join(format!("Godot_v{GODOT_TAG}_win64.exe"))
    } else if cfg!(target_os = "macos") {
        dir.join(format!(
            "Godot_v{GODOT_TAG}_macos.app/Contents/MacOS/Godot"
        ))
    } else {
        dir.join(format!("Godot_v{GODOT_TAG}_linux.x86_64"))
    }
}

fn lib_name() -> String {
    if cfg!(target_os = "windows") {
        "mmo_client.dll".to_string()
    } else if cfg!(target_os = "macos") {
        "libmmo_client.dylib".to_string()
    } else {
        "libmmo_client.so".to_string()
    }
}

// ─── Tasks ───────────────────────────────────────────────────────────────────

fn setup(root: &Path) {
    let dir = godot_bin_dir(root);
    fs::create_dir_all(&dir).expect("cannot create .godot_bin/");

    let (zip_name, url) = download_url();
    let zip_path = dir.join(&zip_name);

    if godot_exe(root).exists() {
        println!("✅  Godot {GODOT_VERSION} already present at {}", godot_exe(root).display());
        return;
    }

    println!("⬇️   Downloading Godot {GODOT_VERSION} from:\n    {url}");
    // Use curl (available on Windows 10+ and all modern macOS/Linux)
    run_cmd(
        Command::new("curl")
            .args(["-L", "-o", zip_path.to_str().unwrap(), &url]),
    );

    println!("📦  Extracting…");
    if cfg!(target_os = "windows") {
        run_cmd(
            Command::new("powershell")
                .args(["-Command", &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    zip_path.display(),
                    dir.display()
                )]),
        );
    } else {
        run_cmd(
            Command::new("unzip")
                .args(["-o", zip_path.to_str().unwrap(), "-d", dir.to_str().unwrap()]),
        );
        // Make executable on Unix
        run_cmd(Command::new("chmod").args(["+x", godot_exe(root).to_str().unwrap()]));
    }

    println!("✅  Godot {GODOT_VERSION} ready at {}", godot_exe(root).display());
}

fn download_url() -> (String, String) {
    let base = format!(
        "https://github.com/godotengine/godot/releases/download/{GODOT_TAG}"
    );
    let (zip_name, artifact) = if cfg!(target_os = "windows") {
        let a = format!("Godot_v{GODOT_TAG}_win64.exe.zip");
        (a.clone(), a)
    } else if cfg!(target_os = "macos") {
        let a = format!("Godot_v{GODOT_TAG}_macos.universal.zip");
        (a.clone(), a)
    } else {
        let a = format!("Godot_v{GODOT_TAG}_linux.x86_64.zip");
        (a.clone(), a)
    };
    (zip_name, format!("{base}/{artifact}"))
}

fn build_gdextension(root: &Path, release: bool) {
    let rust_dir = root.join("rust");
    println!("🔨  Compiling GDExtension ({})…", if release { "release" } else { "debug" });
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&rust_dir).args(["build", "--package", "mmo_client"]);
    if release {
        cmd.arg("--release");
    }
    run_cmd(&mut cmd);
}

fn copy_lib(root: &Path) {
    let lib = lib_name();
    let profile_dir = if cfg!(debug_assertions) { "debug" } else { "release" };
    let src = root.join("rust").join("target").join(profile_dir).join(&lib);
    let dst = root.join("game").join("bin").join(&lib);
    fs::create_dir_all(root.join("game").join("bin")).ok();
    fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {lib}: {e}"));
    println!("📋  Copied {lib} → game/bin/");
}

fn write_gdextension_file(root: &Path) {
    let lib = lib_name();
    let content = format!(
        r#"[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1

[libraries]
windows.debug.x86_64 = "res://bin/{lib}"
windows.release.x86_64 = "res://bin/{lib}"
linux.debug.x86_64 = "res://bin/{lib}"
linux.release.x86_64 = "res://bin/{lib}"
macos.debug = "res://bin/{lib}"
macos.release = "res://bin/{lib}"
"#
    );
    let out = root.join("game").join("mmo_client.gdextension");
    fs::write(&out, content).expect("write .gdextension");
    println!("📝  Written game/mmo_client.gdextension");
}

fn open_editor(root: &Path) {
    let exe = godot_exe(root);
    assert!(exe.exists(), "Godot not found — run `cargo xtask setup` first");
    let project = root.join("game");
    println!("🚀  Opening Godot editor…");
    Command::new(&exe)
        .args(["--editor", "--path", project.to_str().unwrap()])
        .status()
        .expect("failed to launch Godot editor");
}

fn run_game(root: &Path) {
    let exe = godot_exe(root);
    assert!(exe.exists(), "Godot not found — run `cargo xtask setup` first");
    let project = root.join("game");
    println!("▶️   Launching game…");
    Command::new(&exe)
        .args(["--path", project.to_str().unwrap()])
        .status()
        .expect("failed to launch Godot game");
}

fn export_game(root: &Path) {
    let exe = godot_exe(root);
    assert!(exe.exists(), "Godot not found — run `cargo xtask setup` first");
    let project = root.join("game");
    let output = root.join("builds").join("mmo_client.exe");
    fs::create_dir_all(root.join("builds")).ok();
    println!("📦  Exporting game → builds/…");
    Command::new(&exe)
        .args([
            "--headless",
            "--path", project.to_str().unwrap(),
            "--export-release", "Windows Desktop",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to export game");
    println!("✅  Export complete: {}", output.display());
}

fn run_cmd(cmd: &mut Command) {
    let status = cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit()).status()
        .unwrap_or_else(|e| panic!("failed to run command: {e}"));
    assert!(status.success(), "command failed with status: {status}");
}

fn print_help() {
    println!(
        r#"GodotClient xtask — available commands:

  cargo xtask setup    Download Godot {GODOT_VERSION} and export templates
  cargo xtask editor   Build GDExtension (debug) and open the Godot editor
  cargo xtask run      Build GDExtension (debug) and launch the game
  cargo xtask package  Build GDExtension (release) and export a standalone binary
"#
    );
}

