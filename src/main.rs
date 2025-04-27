use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::api::*;
use alertaemcena::config::env_loader::load_config;
use alertaemcena::config::model::{Config, DebugConfig, EmojiConfig};
use alertaemcena::discord::api::{DiscordAPI, EventsThread};
use lazy_static::lazy_static;
use serenity::all::{ChannelId, GuildChannel};
use std::collections::BTreeMap;
use std::process::exit;
use tracing::{debug, info, instrument, warn};
use alertaemcena::tracing::setup_loki;

lazy_static! {
    pub static ref SAVE_FOR_LATER_EMOJI: char = '🔖';
}

#[tokio::main]
async fn main() {
    setup_loki().await;

    let config = load_config();

    debug!("Loaded {:?}", config);

    let discord = DiscordAPI::default().await;

    if config.debug_config.clear_channel {
        discord.delete_all_messages(&config.teatro_channel_id).await;
        discord.delete_all_messages(&config.artes_channel_id).await;

        if config.debug_config.exit_after_clearing {
            exit(0)
        }
    }

    if !config.debug_config.skip_artes {
        run(&config, &discord, Category::Artes, config.artes_channel_id).await;
    }

    run(
        &config,
        &discord,
        Category::Teatro,
        config.teatro_channel_id,
    )
    .await;
}

async fn run(config: &Config, discord: &DiscordAPI, category: Category, channel_id: ChannelId) {
    let guild = discord.get_guild(channel_id).await;
    let threads = discord.get_channel_threads(&guild, channel_id).await;

    if !config.debug_config.skip_feature_reactions {
        handle_reaction_features(discord, threads, &config.voting_emojis).await;
    }

    let events = AgendaCulturalAPI::get_events_by_month(&category, config.debug_config.event_limit)
        .await
        .unwrap();

    if events.is_empty() {
        panic!("No events found");
    }

    let new_events = filter_new_events_by_thread(discord, &guild, events, channel_id).await;

    send_new_events(
        discord,
        new_events,
        &config.debug_config,
        &config.voting_emojis,
    )
    .await;
}

#[instrument(skip(discord, threads, vote_emojis))]
async fn handle_reaction_features(
    discord: &DiscordAPI,
    threads: Vec<GuildChannel>,
    vote_emojis: &[EmojiConfig; 5],
) {
    for thread in threads {
        let messages = discord
            .get_all_messages(thread.id)
            .await
            .expect("Failed to get messages");

        info!("Tagging save for later and sending votes in DM");

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

            discord
                .send_privately_users_review(&message, vote_emojis)
                .await;
        }
    }
}

#[instrument(skip(discord, new_events, emojis, debug_config), fields(new_events_count = %new_events.len()))]
async fn send_new_events(
    discord: &DiscordAPI,
    new_events: BTreeMap<EventsThread, Vec<Event>>,
    debug_config: &DebugConfig,
    emojis: &[EmojiConfig; 5],
) {
    info!("Sending new events");

    if new_events.is_empty() {
        info!("No new events to send");
        return;
    }

    info!("Found {} new events", new_events.len());

    if debug_config.skip_sending {
        info!("Skipping sending events");
        return;
    }

    for (thread, events) in new_events {
        for event in events {
            let message = discord.send_event(thread.thread_id, event).await;

            add_feature_reactions(discord, &message, emojis, *SAVE_FOR_LATER_EMOJI).await;
        }
    }
}
