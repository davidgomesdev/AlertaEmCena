use crate::agenda_cultural::model::Category;
use agenda_cultural::api::get_events;
use tracing::info;

mod agenda_cultural;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let events = get_events(10, &Category::Teatro).await.unwrap();

    events.iter().for_each(|event| {
        info!(
            "Got for event {}: {}",
            event.link, event.details.description
        )
    });
}
