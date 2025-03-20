use serenity::all::{ChannelId, Message};
use tracing::info;
use crate::agenda_cultural::api::AgendaCulturalAPI;
use crate::agenda_cultural::model::{Category, Event};
use crate::config::model::EmojiConfig;
use crate::discord::api::DiscordAPI;

pub async fn get_new_events(
    discord: &DiscordAPI,
    category: &Category,
    channel_id: ChannelId,
    event_limit: Option<i32>,
) -> Vec<Event> {
    let events = AgendaCulturalAPI::get_events(category, event_limit)
        .await
        .unwrap();

    if events.is_empty() {
        panic!("No events found");
    }

    let sent_events = discord.get_event_urls_sent(channel_id).await;

    info!("Channel has {} sent events", sent_events.len());

    let unsent_events: Vec<Event> = events
        .into_iter()
        .filter(|event| !sent_events.contains(&event.link))
        .collect();
    unsent_events
}

pub async fn add_feature_reactions(discord: &DiscordAPI, message: &Message, voting_emojis: &[EmojiConfig; 5], save_for_later_emoji: char) {
    for emoji in voting_emojis {
        discord.add_custom_reaction(message, emoji).await;
    }

    discord
        .add_reaction_to_message(message, save_for_later_emoji)
        .await;
}
