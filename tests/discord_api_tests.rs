use alertaemcena::agenda_cultural::model::*;
use alertaemcena::discord::api::DiscordAPI;
use lazy_static::lazy_static;
use serenity::all::ChannelId;
use std::env;
use uuid::Uuid;

lazy_static! {
    static ref token: String = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    static ref tester_token: String =
        env::var("DISCORD_TESTER_TOKEN").expect("DISCORD_TESTER_TOKEN not set");
    static ref channel_id: ChannelId = env::var("DISCORD_CHANNEL_ID")
        .expect("DISCORD_CHANNEL_ID not set")
        .parse()
        .expect("DISCORD_CHANNEL_ID is in a wrong format");
    static ref SAVE_FOR_LATER_EMOJI: char = '🔖';
}

#[test_log::test(tokio::test)]
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
            tags: vec!["festival".to_string()],
        })
        .await;
}

#[test_log::test(tokio::test)]
async fn should_add_reaction_to_event() {
    let api = build_api_without_cache().await;

    api.delete_all_messages(*channel_id).await;

    let message = api
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
            tags: vec!["festival".to_string()],
        })
        .await;

    api.add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
        .await;
}

#[test_log::test(tokio::test)]
async fn should_read_events() {
    build_api().await.get_event_urls_sent(*channel_id).await;
}

#[test_log::test(tokio::test)]
async fn should_read_event_sent() {
    let (link, unique_event) = generate_random_event();

    let api = build_api().await;

    api.send_event(*channel_id, unique_event).await;

    let is_event_sent = api.get_event_urls_sent(*channel_id).await.contains(&link);

    assert!(is_event_sent);
}

#[test_log::test(tokio::test)]
async fn when_an_event_is_deleted_should_not_read_afterwards() {
    let (link, unique_event) = generate_random_event();

    let api = build_api().await;

    api.send_event(*channel_id, unique_event).await;

    api.delete_all_messages(*channel_id).await;

    let is_event_sent = api.get_event_urls_sent(*channel_id).await.contains(&link);

    assert!(!is_event_sent);
}

#[test_log::test(tokio::test)]
async fn when_someone_react_with_save_later_should_add_that_person_to_message() {
    let (link, unique_event) = generate_random_event();

    let api = build_api().await;

    let mut message = api.send_event(*channel_id, unique_event).await;
    api.add_reaction_to_message(&message, '🔖').await;

    let tester_api = build_tester_api().await;

    tester_api.add_reaction_to_message(&message, '🔖').await;
    api.tag_save_for_later_reactions(&mut message, '🔖').await;

    let message = tester_api
        .get_messages(*channel_id)
        .await
        .into_iter()
        .find(|msg| {
            let embed_url = msg
                .embeds
                .iter()
                .flat_map(|embed| embed.url.clone())
                .collect::<Vec<String>>()
                .pop();

            match embed_url {
                None => false,
                Some(embed_url) => embed_url.contains(&link.clone()),
            }
        })
        .unwrap();

    let saved_later = message
        .content;

    assert!(saved_later
        .contains(tester_api.own_user.id.to_string().as_str()));
    assert!(!saved_later
        .contains(api.own_user.id.to_string().as_str()));
}

fn generate_random_event() -> (String, Event) {
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
        tags: vec!["festival".to_string()],
    };
    (link, unique_event)
}

async fn build_api() -> DiscordAPI {
    DiscordAPI::new(&token, true).await
}

async fn build_api_without_cache() -> DiscordAPI {
    DiscordAPI::new(&token, false).await
}

async fn build_tester_api() -> DiscordAPI {
    DiscordAPI::new(&tester_token, false).await
}
