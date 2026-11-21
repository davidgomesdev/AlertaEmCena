use alertaemcena::config::env_loader::load_config;
use alertaemcena::discord::api::DiscordAPI;
use alertaemcena::discord::backup::count_user_votes;
use alertaemcena::tracing::setup_tracing;
use serenity::all::UserId;

/// Goes through a user's DMs with the bot and prints how many votes each rating got.
///
/// Usage: `vote_summary <discord_user_id>` (or set `VOTE_SUMMARY_USER_ID`)
#[tokio::main]
async fn main() {
    let tracing_handles = setup_tracing().await;

    let user_id: UserId = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("VOTE_SUMMARY_USER_ID").ok())
        .expect("Pass the Discord user ID as first argument or set VOTE_SUMMARY_USER_ID")
        .parse()
        .expect("User ID must be a valid Discord user ID");

    let config = load_config();
    let discord = DiscordAPI::default().await;

    match count_user_votes(&discord, user_id, &config.voting_emojis).await {
        Some(counts) => {
            let total: u32 = counts.iter().sum();
            let percent = |count: u32| {
                if total == 0 {
                    0
                } else {
                    (count as f64 * 100.0 / total as f64).round() as u32
                }
            };

            for (index, count) in counts.iter().enumerate() {
                println!("- {}: {} votes ({}%)", index + 1, count, percent(*count));
            }

            let positive: u32 = counts[2..].iter().sum();

            println!("Total: {} votes", total);
            println!(
                "Positive (>= 3): {} votes ({}%)",
                positive,
                percent(positive)
            );
        }
        None => eprintln!("Failed to count votes, check the logs"),
    }

    tracing_handles.shutdown().await;
}
