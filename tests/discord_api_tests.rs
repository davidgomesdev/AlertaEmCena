use alertaemcena::agenda_cultural::model::*;
use alertaemcena::discord::api::DiscordAPI;
use lazy_static::lazy_static;
use serenity::all::ChannelId;
use std::env;
use uuid::Uuid;

lazy_static! {
    static ref token: String = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    static ref channel_id: ChannelId = env::var("DISCORD_CHANNEL_ID")
        .expect("DISCORD_CHANNEL_ID not set")
        .parse()
        .expect("DISCORD_CHANNEL_ID is in a wrong format");
}

#[tokio::test]
#[test_log::test]
async fn should_send_event() {
    build_api()
        .await
        .send_event(*channel_id, Event {
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

#[tokio::test]
#[test_log::test]
async fn should_read_events() {
    build_api().await.get_event_urls_sent(*channel_id).await;
}

#[tokio::test]
#[test_log::test]
async fn should_read_event_sent() {
    let test_id = Uuid::new_v4();
    let link = format!(
        "https://example.com/events/barca_infern?test_id={}",
        test_id
    );
    let unique_event = Event {
        title: "O Auto da Barca do Inferno".to_string(),
        details: EventDetails {
            subtitle: "Uma peça de Gil Vicente".to_string(),
            description:
                "Uma comédia dramática que reflete sobre os vícios humanos e as escolhas morais."
                    .to_string(),
            image_url: "https://www.culturalkids.pt/wp-content/uploads/2021/04/Auto-01_1.jpg"
                .to_string(),
        },
        link: link.to_string(),
        occurring_at: Schedule {
            dates: "21 setembro 2024 a 23 fevereiro 2025".to_string(),
            times: "qui: 21h; sex: 21h; sáb: 21h; dom: 17h".to_string(),
        },
        venue: "Teatro Nacional D. Maria II, Lisboa".to_string(),
    };

    let api = build_api().await;

    api.send_event(*channel_id, unique_event).await;

    let is_event_sent = api.get_event_urls_sent(*channel_id).await.contains(&link);

    assert!(is_event_sent);
}

#[tokio::test]
#[test_log::test]
async fn when_an_event_is_deleted_should_not_read_afterwards() {
    let test_id = Uuid::new_v4();
    let link = format!(
        "https://example.com/events/barca_infern?test_id={}",
        test_id
    );
    let unique_event = Event {
        title: "O Auto da Barca do Inferno - Test to delete".to_string(),
        details: EventDetails {
            subtitle: "Uma peça de Gil Vicente".to_string(),
            description:
            "Uma comédia dramática que reflete sobre os vícios humanos e as escolhas morais."
                .to_string(),
            image_url: "https://www.culturalkids.pt/wp-content/uploads/2021/04/Auto-01_1.jpg"
                .to_string(),
        },
        link: link.to_string(),
        occurring_at: Schedule {
            dates: "21 setembro 2024 a 23 fevereiro 2025".to_string(),
            times: "qui: 21h; sex: 21h; sáb: 21h; dom: 17h".to_string(),
        },
        venue: "Teatro Nacional D. Maria II, Lisboa".to_string(),
    };

    let api = build_api().await;

    api.send_event(*channel_id, unique_event).await;

    api.delete_all_messages(*channel_id).await;

    let is_event_sent = api.get_event_urls_sent(*channel_id).await.contains(&link);

    assert!(!is_event_sent);
}

async fn build_api() -> DiscordAPI {
    DiscordAPI::new(&token).await
}
