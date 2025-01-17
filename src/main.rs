use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::config::env_loader::load_config;
use alertaemcena::discord::api::DiscordAPI;
use serenity::all::ChannelId;
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

    send_new_events(&discord, &Category::Teatro, config.teatro_channel_id).await;
    send_new_events(&discord, &Category::Artes, config.artes_channel_id).await;
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
