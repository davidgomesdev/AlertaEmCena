use crate::api::model::Category;
use api::api::get_events;
use tracing::info;

mod api;

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
