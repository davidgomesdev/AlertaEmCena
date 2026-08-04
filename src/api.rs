use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use crate::discord::api::{DiscordAPI, EventsThread};
use chrono::NaiveDate;
use serenity::all::{ChannelId, GuildChannel, Message, PartialGuild};
use std::collections::{BTreeMap, HashSet};
use tracing::{debug, info, trace};

pub async fn filter_new_events_by_thread(
    discord: &DiscordAPI,
    guild: &PartialGuild,
    events_by_month: BTreeMap<NaiveDate, Vec<Event>>,
    channel_id: ChannelId,
) -> BTreeMap<EventsThread, Vec<Event>> {
    trace!("Getting threads");
    let threads = discord.get_channel_threads(guild, channel_id).await;

    trace!("Sorting threads by month");
    let threads_by_month =
        get_threads_by_month(discord, channel_id, &events_by_month, &threads).await;

    trace!("Getting sent events");
    let sent_events = get_sent_events(discord, &threads).await;
    // the agenda cultural API has returned the same event twice before for the same month
    // but we're dedup'ing across months, just in case
    let mut unique_unsent_events = HashSet::new();

    let events_by_thread: BTreeMap<EventsThread, Vec<Event>> = threads_by_month
        .into_iter()
        .map(|(date, thread)| {
            let unsent_events = events_by_month[&date]
                .iter()
                .filter(|&e| !sent_events.contains(&e.link))
                .filter(|event| unique_unsent_events.insert(event.link.clone()))
                .cloned()
                .collect();

            (thread, unsent_events)
        })
        .collect();

    info!(
        "{} of the {} fetched events are new ({} already sent)",
        events_by_thread.values().map(Vec::len).sum::<usize>(),
        events_by_month.values().map(Vec::len).sum::<usize>(),
        sent_events.len()
    );

    events_by_thread
}

async fn get_sent_events(discord: &DiscordAPI, threads: &[GuildChannel]) -> HashSet<String> {
    let mut sent_events = HashSet::new();

    for thread in threads.iter() {
        let thread_events = discord.get_event_urls_sent(thread.id).await;

        debug!(
            "Thread '{}' has {} sent events",
            thread.name,
            thread_events.len()
        );

        sent_events.extend(thread_events);
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
