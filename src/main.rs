use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::discord::api::DiscordAPI;
use serenity::all::ChannelId;
use std::env;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let teatro_channel_id: ChannelId = env::var("DISCORD_TEATRO_CHANNEL_ID")
        .expect("DISCORD_TEATRO_CHANNEL_ID not set")
        .parse()
        .unwrap();
    let artes_channel_id: ChannelId = env::var("DISCORD_ARTES_CHANNEL_ID")
        .expect("DISCORD_ARTES_CHANNEL_ID not set")
        .parse()
        .unwrap();

    let discord = DiscordAPI::default().await;

    send_new_events(&discord, &Category::Teatro, teatro_channel_id).await;
    send_new_events(&discord, &Category::Artes, artes_channel_id).await;
}

async fn send_new_events(discord: &DiscordAPI, category: &Category, channel_id: ChannelId) {
    let events = AgendaCulturalAPI::get_events(10, category).await.unwrap();
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

    for event in unsent_events {
        info!("Sending unsent event: '{}' ({})", event.title, event.link);
        discord.send_event(channel_id, event).await;
    }
}
