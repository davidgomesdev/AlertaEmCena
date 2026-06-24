use alertaemcena::config::env_loader::load_config;
use alertaemcena::discord::api::DiscordAPI;
use alertaemcena::tracing::setup_tracing;
use serde::Deserialize;
use serenity::all::UserId;
use std::fs;
use tracing::{error, info, info_span, warn, Instrument};

#[derive(Deserialize)]
struct ReviewRecord {
    url: String,
    rating: u8,
    comment: String,
}

#[tokio::main]
async fn main() {
    let tracing_handles = setup_tracing().await;

    let user_id: UserId = std::env::var("BACKFILL_USER_ID")
        .expect("BACKFILL_USER_ID not set")
        .parse()
        .expect("BACKFILL_USER_ID must be a valid Discord user ID");

    let input_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "scripts/reviews_data.json".to_string());

    let raw = fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("Failed to read '{}': {}", input_path, e));
    let records: Vec<ReviewRecord> =
        serde_json::from_str(&raw).expect("Failed to parse input JSON");

    let config = load_config();
    let discord = DiscordAPI::default().await;
    let channel_ids = [config.teatro_channel_id, config.artes_channel_id];
    let total = records.len();

    async {
        for record in records {
            if !(1..=5).contains(&record.rating) {
                warn!("Skipping '{}': invalid rating {}", record.url, record.rating);
                continue;
            }

            let comment = record.comment.trim();
            let comment = if comment.is_empty() { None } else { Some(comment) };
            let vote_emoji = &config.voting_emojis[(record.rating - 1) as usize];

            match discord
                .send_backfill_review(user_id, &record.url, vote_emoji, comment, &channel_ids)
                .await
            {
                Ok(true) => info!("Sent review for {}", record.url),
                Ok(false) => warn!("Skipped (already sent or event not found): {}", record.url),
                Err(_) => error!("Failed to send review for {}", record.url),
            }
        }
    }
    .instrument(info_span!("backfill_reviews", user_id = %user_id, total))
    .await;

    tracing_handles.shutdown().await;
}
