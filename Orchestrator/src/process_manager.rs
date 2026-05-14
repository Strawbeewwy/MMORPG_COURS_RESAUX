use crate::config::OrchestratorConfig;
use anyhow::{Context, Result};
use std::{
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU16, Ordering},
};
use tokio::sync::Mutex;
use tracing::warn;

pub struct ProcessManager {
    next_port: AtomicU16,
    children: Mutex<Vec<Child>>,
}

impl ProcessManager {
    pub fn new(first_port: u16) -> Self {
        Self {
            next_port: AtomicU16::new(first_port),
            children: Mutex::new(Vec::new()),
        }
    }

    pub async fn spawn_server(&self, config: &OrchestratorConfig) -> Result<u16> {
        let port = self.next_port.fetch_add(1, Ordering::SeqCst);

        let child = Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg(&config.ds_binary)
            .env("DS_PORT", port.to_string())
            .env("DS_IP", "127.0.0.1")
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

        self.children.lock().await.push(child);

        Ok(port)
    }

    pub async fn reap_finished_processes(&self) {
        let mut children = self.children.lock().await;

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
