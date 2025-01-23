use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::config::env_loader::load_config;
use alertaemcena::config::model::EmojiConfig;
use alertaemcena::discord::api::DiscordAPI;
use serenity::all::{ChannelId, Message};
use tracing::{info, instrument};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = load_config();

    let discord = DiscordAPI::default().await;

    if config.debug_config.clear_channel {
        discord.delete_all_messages(config.teatro_channel_id).await;
        discord.delete_all_messages(config.artes_channel_id).await;
    }

    send_new_events(
        &discord,
        &Category::Teatro,
        config.teatro_channel_id,
        config.debug_config.event_limit,
        &config.voting_emojis,
    )
    .await;
    send_new_events(
        &discord,
        &Category::Artes,
        config.artes_channel_id,
        config.debug_config.event_limit,
        &config.voting_emojis,
    )
    .await;
}

#[instrument(skip(discord, channel_id, emojis), fields(channel_id = %channel_id.to_string()))]
async fn send_new_events(
    discord: &DiscordAPI,
    category: &Category,
    channel_id: ChannelId,
    event_limit: Option<i32>,
    emojis: &[EmojiConfig; 5],
) {
    let new_events = get_new_events(discord, category, channel_id, event_limit).await;

    if new_events.is_empty() {
        info!("No new events to send");
        return;
    }

    info!("Found {} new events", new_events.len());

    for event in new_events {
        let message = discord.send_event(channel_id, event).await;

        add_voting_reactions(discord, &message, emojis).await;
    }
}

async fn get_new_events(
    discord: &DiscordAPI,
    category: &Category,
    channel_id: ChannelId,
    event_limit: Option<i32>,
) -> Vec<Event> {
    let events = AgendaCulturalAPI::get_events(category, event_limit)
        .await
        .unwrap();
    let sent_events = discord.get_event_urls_sent(channel_id).await;

    info!("Channel has {} sent events", events.len());

    let unsent_events: Vec<Event> = events
        .into_iter()
        .filter(|event| !sent_events.contains(&event.link))
        .collect();
    unsent_events
}

async fn add_voting_reactions(discord: &DiscordAPI, message: &Message, emojis: &[EmojiConfig; 5]) {
    for emoji in emojis {
        discord.vote_message(message, emoji).await;
    }
}
