use crate::config::BrokerConfig;
use crate::net::network_event::BrokerNetwork;
use crate::pubsub::state::PubSubState;
use std::time::Duration;

pub struct BrokerApp {
    config: BrokerConfig,
    network: BrokerNetwork,
    pubsub_state: PubSubState,
}

impl BrokerApp {
    pub fn new(config: BrokerConfig) -> anyhow::Result<Self> {
        let network = BrokerNetwork::listen(config.port)?;

        Ok(Self {
            config,
            network,
            pubsub_state: PubSubState::default(),
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!("broker started");

        let tick_duration = Duration::from_millis(self.config.tick_ms);

        loop {
            self.network.poll_events(&mut self.pubsub_state);
            std::thread::sleep(tick_duration);
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let config = BrokerConfig::from_env();
    let mut app = BrokerApp::new(config)?;

    app.run()
}