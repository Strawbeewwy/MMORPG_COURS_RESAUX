use crate::config::OrchestratorConfig;
use anyhow::{Context, Result};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU16, Ordering},
};
use tokio::sync::Mutex;
use tracing::{info, warn};

struct ManagedChild {
    child: Child,
    shard_id: Option<u32>,
}

pub struct ProcessManager {
    next_port: AtomicU16,//integer safe for multithread access
    children: Mutex<Vec<ManagedChild>>,
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
        self.spawn_server_for_shard(config, None).await
    }

    pub async fn spawn_server_for_shard(
        &self,
        config: &OrchestratorConfig,
        shard_id: Option<u32>,
    ) -> Result<u16> {
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

        let mut command = Command::new(&binary_path);
        command
            .arg(&config.ds_binary) //name of the package
            .env("SHARD_PORT", port.to_string())
            .env("SHARD_IP", "127.0.0.1") //keep has local for now, change later to a real address
            .env("ZONE", &config.zone)
            .env("ORCH_ADDR", config.orch_addr.to_string())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        if let Some(shard_id) = shard_id {
            command.env("SHARD_ID", shard_id.to_string());
        }

        //runs a child process that will run the dedicated server
        let child = command
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn dedicated server package {} on port {} for shard {:?}",
                    config.ds_binary, port, shard_id
                )
            })?;

        //lock the children vector and add the child to it
        self.children.lock().await.push(ManagedChild { child, shard_id });

        Ok(port)
    }

    pub async fn stop_shard_servers(&self, shard_ids: &[u32]) -> Result<usize> {
        let mut children = self.children.lock().await;
        let mut stopped = 0usize;

        for managed in children.iter_mut() {
            let Some(managed_shard_id) = managed.shard_id else {
                continue;
            };

            if !shard_ids.contains(&managed_shard_id) {
                continue;
            }

            match managed.child.kill() {
                Ok(_) => {
                    let _ = managed.child.wait();
                    stopped += 1;
                    info!("stopped dedicated server for shard {}", managed_shard_id);
                }
                Err(error) => {
                    warn!(
                        "failed to kill dedicated server for shard {}: {}",
                        managed_shard_id,
                        error
                    );
                }
            }
        }

        children.retain_mut(|managed| match managed.child.try_wait() {
            Ok(Some(status)) => {
                warn!(
                    "dedicated server process for shard {:?} exited with status {}",
                    managed.shard_id,
                    status
                );
                false
            }
            Ok(None) => true,
            Err(err) => {
                warn!(
                    "failed to check dedicated server process status for shard {:?}: {err}",
                    managed.shard_id
                );
                false
            }
        });

        Ok(stopped)
    }

    pub async fn running_process_count(&self) -> usize {
        self.children.lock().await.len()
    }

    pub async fn reap_finished_processes(&self) {
        //lock the children vector so we don't add new ones while we are removing old ones
        let mut children = self.children.lock().await;

        //retain only the children that are still running
        children.retain_mut(|managed| match managed.child.try_wait() {
            Ok(Some(status)) => {
                warn!(
                    "dedicated server process for shard {:?} exited with status {}",
                    managed.shard_id,
                    status
                );
                false
            }
            Ok(None) => true,
            Err(err) => {
                warn!(
                    "failed to check dedicated server process status for shard {:?}: {err}",
                    managed.shard_id
                );
                false
            }
        });
    }
}
