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
            for (index, count) in counts.iter().enumerate() {
                println!(
                    "{} ({}): {} votes",
                    index + 1,
                    config.voting_emojis[index],
                    count
                );
            }
            println!("Total: {} votes", counts.iter().sum::<u32>());
        }
        None => eprintln!("Failed to count votes, check the logs"),
    }

    tracing_handles.shutdown().await;
}
