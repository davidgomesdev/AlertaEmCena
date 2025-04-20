use crate::agenda_cultural::api::AgendaCulturalAPI;
use crate::agenda_cultural::model::{Category, Event};
use crate::config::model::EmojiConfig;
use crate::discord::api::{DiscordAPI, EventsThread};
use serenity::all::{ChannelId, Message};
use std::collections::BTreeMap;
use tracing::info;

pub async fn get_new_events_by_thread(
    discord: &DiscordAPI,
    category: &Category,
    channel_id: ChannelId,
    event_limit: Option<i32>,
) -> BTreeMap<EventsThread, Vec<Event>> {
    let events = AgendaCulturalAPI::get_events(category, event_limit)
        .await
        .unwrap();

    if events.is_empty() {
        panic!("No events found");
    }

    let mut threads = BTreeMap::new();
    let mut sent_events = Vec::new();

    let guild = discord.get_guild(channel_id).await;
    let active_threads = discord.get_channel_active_threads(&guild, channel_id).await;

    for date in events.keys() {
        let thread = discord
            .get_date_thread(&active_threads, channel_id, *date)
            .await;

        let mut thread_events = discord.get_event_urls_sent(thread.channel_id).await;

        info!("Channel has {} sent events", sent_events.len());
        sent_events.append(&mut thread_events);

        threads.insert(date, thread);
    }

    threads
        .into_iter()
        .map(|(date, thread)| {
            let unsent_events = events[date]
                .iter()
                .filter(|&e| !sent_events.contains(&e.link))
                .cloned()
                .collect();

            (thread, unsent_events)
        })
        .collect()
}

pub async fn add_feature_reactions(
    discord: &DiscordAPI,
    message: &Message,
    voting_emojis: &[EmojiConfig; 5],
    save_for_later_emoji: char,
) {
    for emoji in voting_emojis {
        discord.add_custom_reaction(message, emoji).await;
    }

    discord
        .add_reaction_to_message(message, save_for_later_emoji)
        .await;
}
