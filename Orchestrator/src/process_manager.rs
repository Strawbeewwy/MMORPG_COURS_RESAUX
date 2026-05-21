use crate::config::OrchestratorConfig;
use anyhow::{Context, Result};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU16, Ordering},
};
use tokio::sync::Mutex;
use tracing::warn;

pub struct ProcessManager {
    next_port: AtomicU16,//integer safe for multithread access
    children: Mutex<Vec<Child>>,
}

impl ProcessManager {
    pub fn new(first_port: u16) -> Self {
        Self {
            next_port: AtomicU16::new(first_port),
            children: Mutex::new(Vec::new()),
        }
    }

    pub async fn spawn_server(&self, config: &OrchestratorConfig)
        -> Result<u16> {
        //fetch the port and increment it
        let port = self.next_port.fetch_add(1, Ordering::SeqCst);


        let binary_name = if cfg!(windows) && !config.ds_binary.to_ascii_lowercase().ends_with(".exe") {
            format!("{}.exe", config.ds_binary)
        } else {
            config.ds_binary.clone()
        };

        // expects workspace build output location
        let binary_path: PathBuf = PathBuf::from("target")
            .join("debug")
            .join(&binary_name);


        //runs a child process that will run the dedicated server
        let child = Command::new(&binary_path)
            .arg(&config.ds_binary) //name of the package
            .env("DS_PORT", port.to_string())
            .env("DS_IP", "127.0.0.1")//keep has local for now, change later to a real address
            .env("DS_ZONE", &config.zone)
            .env("ORCH_ADDR", config.orch_addr.to_string())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn dedicated server package {} on port {}",
                    config.ds_binary, port
                )
            })?;

        //lock the children vector and add the child to it
        self.children.lock().await.push(child);

        Ok(port)
    }

    pub async fn running_process_count(&self) -> usize {
        self.children.lock().await.len()
    }
    pub async fn reap_finished_processes(&self) {
        //lock the children vector so we don't add new ones while we are removing old ones
        let mut children = self.children.lock().await;

        //retain only the children that are still running
        children.retain_mut(|child| match child.try_wait() {
            Ok(Some(status)) => {
                warn!("dedicated server process exited with status {}", status);
                false
            }
            Ok(None) => true,
            Err(err) => {
                warn!("failed to check dedicated server process status: {err}");
                false
            }
        });
    }
}
