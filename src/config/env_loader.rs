use crate::config::model::{Config, DebugConfig};
use serenity::all::ChannelId;
use std::env;

pub fn load_config() -> Config {
    let teatro_channel_id: ChannelId = load_channel_id_config("DISCORD_TEATRO_CHANNEL_ID");
    let artes_channel_id: ChannelId = load_channel_id_config("DISCORD_ARTES_CHANNEL_ID");

    let debug_clear_channel = load_bool_config("DEBUG_CLEAR_CHANNEL", false);
    let debug_event_limit = load_i32_config("DEBUG_EVENT_LIMIT");

    Config {
        debug_config: DebugConfig {
            clear_channel: debug_clear_channel,Z
            event_limit: debug_event_limit,
        },
        teatro_channel_id,
        artes_channel_id,
    }
}

fn load_channel_id_config(name: &str) -> ChannelId {
    env::var(name)
        .unwrap_or_else(|_| panic!("{} must be set.", name))
        .parse()
        .unwrap_or_else(|_| panic!("{} is not a valid Discord channel ID", name))
}

fn load_bool_config(name: &str, default: bool) -> bool {
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .unwrap_or_else(|_| {
            panic!(
                "Invalid config '{}'. Expected either 'true' or 'false'",
                name
            )
        })
}

fn load_i32_config(name: &str) -> Option<i32> {
    match env::var(name) {
        Ok(value) => {
            Some(value.parse().unwrap_or_else(|_| {
                panic!("Invalid config '{}'. Expected an integer number.", name)
            }))
        }
        Err(_) => None,
    }
}
