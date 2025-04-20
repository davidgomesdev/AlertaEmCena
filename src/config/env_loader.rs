use crate::config::model::{Config, DebugConfig, EmojiConfig};
use serenity::all::ChannelId;
use std::env;

pub fn load_config() -> Config {
    let teatro_channel_id: ChannelId = load_channel_id_config("DISCORD_TEATRO_CHANNEL_ID");
    let artes_channel_id: ChannelId = load_channel_id_config("DISCORD_ARTES_CHANNEL_ID");
    let voting_emojis: [EmojiConfig; 5] = load_voting_emojis_config("VOTING_EMOJIS");

    let debug_config = DebugConfig {
        clear_channel: load_bool_config("DEBUG_CLEAR_CHANNEL", false),
        exit_after_clearing: load_bool_config("DEBUG_EXIT_AFTER_CLEARING", false),
        skip_sending: load_bool_config("DEBUG_SKIP_SENDING", false),
        skip_feature_reactions: load_bool_config("DEBUG_SKIP_FEATURE_REACTIONS", false),
        event_limit: load_i32_config("DEBUG_EVENT_LIMIT"),
    };

    Config {
        debug_config,
        teatro_channel_id,
        artes_channel_id,
        voting_emojis,
    }
}

fn load_channel_id_config(name: &str) -> ChannelId {
    env::var(name)
        .unwrap_or_else(|_| panic!("{} must be set.", name))
        .parse()
        .unwrap_or_else(|_| panic!("{} is not a valid Discord channel ID", name))
}

pub fn load_voting_emojis_config(name: &str) -> [EmojiConfig; 5] {
    let config = env::var(name).unwrap_or_else(|_| panic!("{} must be set.", name));

    let emojis: [&str; 5] = config
        .split(";")
        .collect::<Vec<&str>>()
        .try_into()
        .expect("Expected just 5 semi-colon separated emojis");

    emojis.map(|c| {
        let split = c
            .split_once(":")
            .expect("Emojis must be comma-separated in the Name:ID format");

        EmojiConfig {
            id: split.1.to_string().parse().unwrap_or_else(|_| {
                panic!(
                    "{} is not a valid Discord channel ID. Must be an integer but got: {}",
                    name, split.1
                )
            }),
            name: split.0.to_string(),
        }
    })
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
