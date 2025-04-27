use lazy_static::lazy_static;
use std::{env, io};
use tracing::{info, warn, Level};
use tracing_loki::url::Url;
use tracing_loki::BackgroundTask;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter, fmt};

lazy_static! {
    static ref LOKI_URL: Option<String> = env::var("LOKI_URL").ok();
}

fn build_loki_layer(base_url: Url) -> (tracing_loki::Layer, BackgroundTask) {
    tracing_loki::layer(
        base_url,
        vec![("service".into(), "alertaemcena".into())]
            .into_iter()
            .collect(),
        vec![].into_iter().collect(),
    )
    .unwrap()
}

pub async fn setup_loki() {
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
                    let (layer, task) = build_loki_layer(base_url);

                    registry.with(layer).init();
                    tokio::spawn(task);

                    info!("Loki initialized");
                }
                Err(_) => {
                    registry.init();
                    warn!("Couldn't connect to Loki. Continuing without it.");
                }
            };
        }
    };
}
