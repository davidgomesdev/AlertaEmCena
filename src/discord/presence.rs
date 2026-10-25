use serenity::all::{GatewayIntents, OnlineStatus, ShardManager};
use serenity::Client;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// Keeps a gateway connection open so the bot shows as online while the app is running
pub struct DiscordPresence {
    shard_manager: Arc<ShardManager>,
    gateway_task: JoinHandle<()>,
}

impl DiscordPresence {
    pub async fn go_online(token: &str) -> Option<Self> {
        let mut client = match Client::builder(token, GatewayIntents::empty())
            .status(OnlineStatus::Online)
            .await
        {
            Ok(client) => client,
            Err(err) => {
                error!("Failed creating presence client: {}", err);
                return None;
            }
        };

        let shard_manager = client.shard_manager.clone();
        let gateway_task = tokio::spawn(async move {
            if let Err(err) = client.start().await {
                error!("Gateway connection for presence failed: {}", err);
            }
        });

        info!("Set bot as online");

        Some(Self {
            shard_manager,
            gateway_task,
        })
    }

    pub async fn go_offline(self) {
        self.shard_manager.shutdown_all().await;

        if let Err(err) = self.gateway_task.await {
            error!("Presence gateway task failed: {}", err);
        }

        debug!("Set bot as offline");
    }
}
