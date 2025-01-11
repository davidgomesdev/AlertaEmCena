use alertaemcena::agenda_cultural::api::{AgendaCulturalAPI};
use alertaemcena::agenda_cultural::model::Category;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let events = AgendaCulturalAPI::get_events(10, &Category::Teatro).await.unwrap();

    events.iter().for_each(|event| {
        info!(
            "Got for event {}: {}",
            event.link, event.details.description
        )
    });
}
