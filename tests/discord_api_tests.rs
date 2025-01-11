use alertaemcena::discord::api::DiscordAPI;
use lazy_static::lazy_static;
use std::env;
use alertaemcena::agenda_cultural::model::*;

lazy_static! {
    static ref token: String = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    static ref channelId: String =
        env::var("DISCORD_CHANNEL_ID").expect("DISCORD_CHANNEL_ID not set");
}

#[tokio::test]
async fn test_discord_api() {
    build_api()
        .await
        .send_event(channelId.parse().unwrap(), Event {
            title: "O Auto da Barca do Inferno".to_string(),
            details: EventDetails {
                subtitle: "Uma peça de Gil Vicente".to_string(),
                description: "Uma comédia dramática que reflete sobre os vícios humanos e as escolhas morais.".to_string(),
                image_url: "https://www.culturalkids.pt/wp-content/uploads/2021/04/Auto-01_1.jpg".to_string(),
            },
            link: "https://example.com/events/barca_inferno".to_string(),
            occurring_at: Schedule {
                dates: "21 setembro 2024 a 23 fevereiro 2025".to_string(),
                times: "qui: 21h; sex: 21h; sáb: 21h; dom: 17h".to_string(),
            },
            venue: "Teatro Nacional D. Maria II, Lisboa".to_string(),
        })
        .await;
}

async fn build_api() -> DiscordAPI {
    DiscordAPI::new(&token).await
}
