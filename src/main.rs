use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::discord::api::DiscordAPI;
use futures::future::join_all;
use lazy_static::lazy_static;
use serenity::all::ChannelId;
use std::env;
use tracing::info;

lazy_static! {
    static ref channel_id: ChannelId = env::var("DISCORD_CHANNEL_ID")
        .expect("DISCORD_CHANNEL_ID not set")
        .parse()
        .unwrap();
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let discord = DiscordAPI::default().await;
    let events = AgendaCulturalAPI::get_events(10, &Category::Teatro)
        .await
        .unwrap();
    let sent_events = discord.get_event_urls_sent(*channel_id).await;

    info!("Channel has {} sent events", events.len());

    let unsent_events: Vec<Event> = events
        .into_iter()
        .filter(|event| !sent_events.contains(&event.link))
        .collect();

    if unsent_events.is_empty() {
        info!("No new events to send");
        return;
    }

    join_all(unsent_events.into_iter().map(|event| async {
        info!("Sending unsent event: '{}' ({})", event.title, event.link);
        discord.send_event(*channel_id, event).await;
    }))
    .await;
}
