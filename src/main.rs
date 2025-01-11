use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::discord::api::DiscordAPI;
use serenity::all::ChannelId;
use std::env;
use tracing::{info, instrument};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let teatro_channel_id: ChannelId = env::var("DISCORD_TEATRO_CHANNEL_ID")
        .expect("DISCORD_TEATRO_CHANNEL_ID not set")
        .parse()
        .expect("DISCORD_TEATRO_CHANNEL_ID is not a valid channel ID");
    let artes_channel_id: ChannelId = env::var("DISCORD_ARTES_CHANNEL_ID")
        .expect("DISCORD_ARTES_CHANNEL_ID not set")
        .parse()
        .expect("DISCORD_ARTES_CHANNEL_ID is not a valid channel ID");

    let discord = DiscordAPI::default().await;

    send_new_events(&discord, &Category::Teatro, teatro_channel_id).await;
    send_new_events(&discord, &Category::Artes, artes_channel_id).await;
}

#[instrument(skip(discord, channel_id), fields(channel_id = %channel_id.to_string()))]
async fn send_new_events(discord: &DiscordAPI, category: &Category, channel_id: ChannelId) {
    let events = AgendaCulturalAPI::get_events(12, category).await.unwrap();
    let sent_events = discord.get_event_urls_sent(channel_id).await;

    info!("Channel has {} sent events", events.len());

    let unsent_events: Vec<Event> = events
        .into_iter()
        .filter(|event| !sent_events.contains(&event.link))
        .collect();

    if unsent_events.is_empty() {
        info!("No new events to send");
        return;
    }

    info!("Found {} new events", unsent_events.len());

    for event in unsent_events {
        discord.send_event(channel_id, event).await;
    }
}
