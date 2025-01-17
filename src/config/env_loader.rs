use crate::config::model::{Config, DebugConfig};
use serenity::all::ChannelId;
use std::env;

pub fn load_config() -> Config {
    let teatro_channel_id: ChannelId = load_channel_id_config("DISCORD_TEATRO_CHANNEL_ID");
    let artes_channel_id: ChannelId = load_channel_id_config("DISCORD_ARTES_CHANNEL_ID");

    let debug_delete_all_messages = load_bool_config("DEBUG_DELETE_ALL_MESSAGES", false);

    Config {
        debug_config: DebugConfig {
            delete_all_messages: debug_delete_all_messages,
        },
        teatro_channel_id,
        artes_channel_id,
    }
}

fn load_channel_id_config(name: &str) -> ChannelId {
    env::var(name)
        .expect(format!("{} must be set.", name).as_str())
        .parse()
        .expect(format!("{} is not a valid Discord channel ID", name).as_str())
}

fn load_bool_config(name: &str, default: bool) -> bool {
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .expect(
            format!(
                "Invalid config '{}'. Expected either 'true' or 'false'",
                name
            )
            .as_str(),
        )
}
