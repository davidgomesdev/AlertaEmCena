use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use crate::discord::api::{DiscordAPI, EventsThread};
use chrono::NaiveDate;
use serenity::all::{ChannelId, GuildChannel, Message, PartialGuild};
use std::collections::BTreeMap;
use tracing::info;

pub async fn filter_new_events_by_thread(
    discord: &DiscordAPI,
    guild: &PartialGuild,
    events_by_month: BTreeMap<NaiveDate, Vec<Event>>,
    channel_id: ChannelId,
) -> BTreeMap<EventsThread, Vec<Event>> {
    let active_threads = discord.get_channel_threads(guild, channel_id).await;

    let threads =
        get_threads_by_month(discord, channel_id, &events_by_month, &active_threads).await;
    let sent_events = get_sent_events(discord, &active_threads).await;

    threads
        .into_iter()
        .map(|(date, thread)| {
            let unsent_events = events_by_month[&date]
                .iter()
                .filter(|&e| !sent_events.contains(&e.link))
                .cloned()
                .collect();

            (thread, unsent_events)
        })
        .collect()
}

async fn get_sent_events(discord: &DiscordAPI, threads: &[GuildChannel]) -> Vec<String> {
    let mut sent_events = Vec::new();

    for thread in threads.iter() {
        let mut thread_events = discord.get_event_urls_sent(thread.id).await;

        sent_events.append(&mut thread_events);

        info!(
            "Thread '{}' has {} sent events",
            thread.name,
            sent_events.len()
        );
    }
    sent_events
}

async fn get_threads_by_month(
    discord: &DiscordAPI,
    channel_id: ChannelId,
    events: &BTreeMap<NaiveDate, Vec<Event>>,
    active_threads: &[GuildChannel],
) -> BTreeMap<NaiveDate, EventsThread> {
    let mut threads = BTreeMap::new();

    for date in events.keys() {
        let thread = discord
            .get_date_thread(active_threads, channel_id, *date)
            .await;

        threads.insert(*date, thread);
    }

    threads
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
