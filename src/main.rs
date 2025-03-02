use alertaemcena::agenda_cultural::model::Category;
use alertaemcena::config::env_loader::load_config;
use alertaemcena::config::model::{Config, DebugConfig, EmojiConfig};
use alertaemcena::discord::api::DiscordAPI;
use lazy_static::lazy_static;
use serenity::all::ChannelId;
use std::process::exit;
use tracing::{debug, info, instrument, warn};
use alertaemcena::api::*;

lazy_static! {
    pub static ref SAVE_FOR_LATER_EMOJI: char = '🔖';
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = load_config();

    debug!("Loaded {:#?}", config);

    let discord = DiscordAPI::default().await;

    if config.debug_config.clear_channel {
        discord.delete_all_messages(config.teatro_channel_id).await;
        discord.delete_all_messages(config.artes_channel_id).await;

        if config.debug_config.exit_after_clearing {
            exit(0)
        }
    }

    run(&config, &discord, Category::Artes, config.artes_channel_id).await;
    run(
        &config,
        &discord,
        Category::Teatro,
        config.teatro_channel_id,
    )
    .await;
}

async fn run(config: &Config, discord: &DiscordAPI, category: Category, channel_id: ChannelId) {
    handle_save_for_later_feature(discord, channel_id, &config.voting_emojis).await;

    send_new_events(
        discord,
        &category,
        channel_id,
        &config.debug_config,
        &config.voting_emojis,
    )
    .await;
}

async fn handle_save_for_later_feature(discord: &DiscordAPI, channel_id: ChannelId, vote_emojis: &[EmojiConfig; 5]) {
    let messages = discord.get_all_messages(channel_id).await;

    match messages {
        Ok(messages) => {
            info!("Backfilling save later reaction and mentioning");

            for mut message in messages {
                if message.embeds.is_empty() {
                    warn!(
                        "Found message without embed (id={}; content={})",
                        message.id, message.content
                    );
                    continue;
                }

                discord
                    .add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
                    .await;

                discord
                    .tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI, vote_emojis)
                    .await;
            }
        }
        Err(err) => warn!("Failed to react to a msg due to {}", err),
    }
}

#[instrument(skip(discord, channel_id, emojis), fields(channel_id = %channel_id.to_string()))]
async fn send_new_events(
    discord: &DiscordAPI,
    category: &Category,
    channel_id: ChannelId,
    debug_config: &DebugConfig,
    emojis: &[EmojiConfig; 5],
) {
    info!("Sending new events");

    let new_events = get_new_events(discord, category, channel_id, debug_config.event_limit).await;

    if new_events.is_empty() {
        info!("No new events to send");
        return;
    }

    info!("Found {} new events", new_events.len());

    if debug_config.skip_sending {
        info!("Skipping sending events");
        return;
    }

    for event in new_events {
        let message = discord.send_event(channel_id, event).await;

        add_feature_reactions(discord, &message, emojis, *SAVE_FOR_LATER_EMOJI).await;
    }
}
