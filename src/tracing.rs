use lazy_static::lazy_static;
use std::{env, io};
use tokio::task::JoinHandle;
use tracing::{info, warn, Level};
use tracing_loki::url::Url;
use tracing_loki::{BackgroundTask, BackgroundTaskController};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter, fmt};

lazy_static! {
    static ref LOKI_URL: Option<String> = env::var("LOKI_URL").ok();
}

fn build_loki_layer(
    base_url: Url,
) -> (
    tracing_loki::Layer,
    BackgroundTaskController,
    BackgroundTask,
) {
    tracing_loki::builder()
        .label("service", "alertaemcena")
        .expect("Failed setting label")
        .build_controller_url(base_url)
        .unwrap()
}

pub async fn setup_loki() -> Option<(BackgroundTaskController, JoinHandle<()>)> {
    let filter = filter::Targets::new()
        .with_target("alertaemcena", Level::TRACE)
        .with_default(Level::WARN);

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(io::stdout));

    match LOKI_URL.as_ref() {
        None => {
            registry.init();
            warn!("Loki URL not provided. Continuing without it.");
        }
        Some(base_url) => {
            let base_url: Url = base_url.parse().expect("Invalid URL format");

            match reqwest::get(base_url.clone()).await {
                Ok(_) => {
                    let (layer, controller, task) = build_loki_layer(base_url);

                    registry.with(layer).init();
                    let handle = tokio::spawn(task);

                    info!("Loki initialized");

                    return Some((controller, handle));
                }
                Err(_) => {
                    registry.init();
                    warn!("Couldn't connect to Loki. Continuing without it.");
                }
            };
        }
    };

    None
}
