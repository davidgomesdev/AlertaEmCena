mod discord {
    use alertaemcena::api::add_feature_reactions;
    use alertaemcena::config::env_loader::load_voting_emojis_config;
    use chrono::NaiveDate;
    use helpers::*;
    use lazy_static::lazy_static;
    use serenity::all::{ChannelId, GuildChannel};
    use std::env;
    use std::time::Duration;

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
        let api = build_api().await;
        send_random_event(&api).await;
    }

    #[test_log::test(tokio::test)]
    async fn should_add_reaction_to_event() {
        let api = build_api_without_cache().await;
        let (_, _, message) = send_random_event(&api).await;

        api.add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_read_events() {
        let api = build_api().await;
        let (thread_id, link, _) = send_random_event(&api).await;

        let size = api
            .get_event_urls_sent(thread_id)
            .await
            .into_iter()
            .filter(|msg| *msg == link)
            .collect::<Vec<String>>()
            .len();

        assert_eq!(size, 1);
    }

    #[test_log::test(tokio::test)]
    async fn should_read_event_sent() {
        let api = build_api().await;
        let (thread_id, link, _) = send_random_event(&api).await;

        let is_event_sent = api.get_event_urls_sent(thread_id).await.contains(&link);

        assert!(is_event_sent);
    }

    #[test_log::test(tokio::test)]
    async fn when_an_event_is_deleted_should_not_read_afterwards() {
        let api = build_api().await;
        let (thread_id, link, message) = send_random_event(&api).await;

        message
            .delete(&api.client.http)
            .await
            .expect("Failed deleting event sent");

        let is_event_sent = api.get_event_urls_sent(thread_id).await.contains(&link);

        assert!(!is_event_sent);
    }

    #[test_log::test(tokio::test)]
    async fn when_someone_reacts_with_save_later_should_add_that_person_to_message() {
        let api = build_api().await;
        let (thread_id, link, mut message) = send_random_event(&api).await;
        api.add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;

        let tester_api = build_tester_api().await;
        let voting_emojis = load_voting_emojis_config("VOTING_EMOJIS");

        tester_api
            .add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;
        api.tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI, &voting_emojis)
            .await;

        let message = tester_api
            .get_messages(thread_id)
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

        let saved_later = message.content;

        assert!(saved_later.contains(tester_api.own_user.id.to_string().as_str()));
        assert!(!saved_later.contains(api.own_user.id.to_string().as_str()));
    }

    #[test_log::test(tokio::test)]
    async fn when_someone_removes_save_for_later_react_should_add_remove_that_person_from_the_message(
    ) {
        let api = build_api().await;
        let (thread_id, _, mut message) = send_random_event(&api).await;

        api.add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;

        let tester_api = build_tester_api().await;
        let voting_emojis = load_voting_emojis_config("VOTING_EMOJIS");

        tester_api
            .add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;
        api.tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI, &voting_emojis)
            .await;

        message
            .delete_reaction_emoji(&tester_api.client.http, *SAVE_FOR_LATER_EMOJI)
            .await
            .unwrap();
        api.tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI, &voting_emojis)
            .await;

        let message = tester_api
            .client
            .http
            .clone()
            .get_message(thread_id, message.id)
            .await
            .unwrap();

        let saved_later = message.content;

        assert!(!saved_later.contains(tester_api.own_user.id.to_string().as_str()));
        assert!(!saved_later.contains(api.own_user.id.to_string().as_str()));
    }

    #[test_log::test(tokio::test)]
    async fn should_send_the_voted_event_message_via_dm_only_once() {
        let api = build_api().await;
        let (_, _, message) = send_random_event(&api).await;

        let voting_emojis = load_voting_emojis_config("VOTING_EMOJIS");

        add_feature_reactions(&api, &message, &voting_emojis, *SAVE_FOR_LATER_EMOJI).await;

        let tester_api = build_tester_api().await;

        tester_api
            .add_custom_reaction(&message, &voting_emojis[3])
            .await;

        // allows manual testing - bots can't vote on each other
        tokio::time::sleep(Duration::from_secs(5)).await;

        api.send_privately_users_review(&message, &voting_emojis)
            .await;
    }

    #[test_log::test(tokio::test)]
    async fn when_someone_saves_for_later_reacts_with_a_three_vote_should_remove_the_user_from_interested(
    ) {
        let api = build_api().await;
        let (thread_id, _, mut message) = send_random_event(&api).await;
        let voting_emojis = load_voting_emojis_config("VOTING_EMOJIS");

        add_feature_reactions(&api, &message, &voting_emojis, *SAVE_FOR_LATER_EMOJI).await;

        let tester_api = build_tester_api().await;

        tester_api
            .add_reaction_to_message(&message, *SAVE_FOR_LATER_EMOJI)
            .await;
        tester_api
            .add_custom_reaction(&message, &voting_emojis[2])
            .await;

        api.tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI, &voting_emojis)
            .await;

        let message = tester_api
            .client
            .http
            .get_message(thread_id, message.id)
            .await
            .expect("Failed getting sent message");
        let saved_later = message.content;

        assert!(!saved_later.contains(tester_api.own_user.id.to_string().as_str()));
        assert!(!saved_later.contains(api.own_user.id.to_string().as_str()));
    }

    #[test_log::test(tokio::test)]
    async fn should_get_threads_of_only_the_specified_channel() {
        let api = build_api().await;
        let guild = api.get_guild(*channel_id).await;
        let active_threads = api.get_channel_threads(&guild, *channel_id).await;

        assert!(
            active_threads
                .iter()
                .all(|thread| thread.parent_id == Some(*channel_id)),
            "Didn't get threads of only the specified channel!"
        );
    }

    #[test_log::test(tokio::test)]
    async fn should_create_date_thread() {
        let api = build_api().await;
        let thread_date = NaiveDate::from_ymd_opt(2021, 3, 12).unwrap();
        let thread_name = "Março 2021";

        let guild = api.get_guild(*channel_id).await;
        let active_threads = api.get_channel_threads(&guild, *channel_id).await;

        api.get_date_thread(&active_threads, *channel_id, thread_date)
            .await;

        let mut threads = api
            .get_channel_threads(&guild, *channel_id)
            .await
            .into_iter()
            .filter(|thread| thread.name == thread_name)
            .collect::<Vec<GuildChannel>>();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads.pop().unwrap().name, thread_name);
    }

    #[test_log::test(tokio::test)]
    async fn should_not_create_duplicate_date_thread() {
        let api = build_api_without_cache().await;
        let guild = api.get_guild(*channel_id).await;
        let thread_date = NaiveDate::from_ymd_opt(1999, 3, 12).unwrap();
        let thread_name = "Março 1999";

        let active_threads = api.get_channel_threads(&guild, *channel_id).await;
        let date_thread = api
            .get_date_thread(&active_threads, *channel_id, thread_date)
            .await;

        let active_threads = api.get_channel_threads(&guild, *channel_id).await;
        let second_date_thread = api
            .get_date_thread(&active_threads, *channel_id, thread_date)
            .await;

        assert_eq!(
            date_thread.thread_id.get(),
            second_date_thread.thread_id.get()
        );

        let mut threads = api
            .get_channel_threads(&guild, *channel_id)
            .await
            .into_iter()
            .filter(|thread| thread.name == thread_name)
            .collect::<Vec<GuildChannel>>();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads.pop().unwrap().name, thread_name);
    }

    mod end_to_end {
        use crate::discord::channel_id;
        use crate::discord::helpers::{build_api, generate_random_event, send_event};
        use alertaemcena::agenda_cultural::model::Event;
        use alertaemcena::api::filter_new_events_by_thread;
        use alertaemcena::discord::api::EventsThread;
        use chrono::{NaiveDate, TimeDelta};
        use std::collections::BTreeMap;
        use std::ops::Add;

        #[test_log::test(tokio::test)]
        async fn api_should_filter_sent_events() {
            let api = build_api().await;
            let original_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
            let random_event = generate_random_event();
            let link = random_event.0;
            let events: BTreeMap<NaiveDate, Vec<Event>> =
                BTreeMap::from([(original_date, Vec::from([random_event.1.clone()]))]);

            let events = filter_new_events_by_thread(
                &api,
                &api.get_guild(*channel_id).await,
                events,
                *channel_id,
            )
            .await;

            let find_result = find_new_unique_event(&link, events);

            assert!(find_result.is_some());

            send_event(&api, random_event.1.clone(), original_date).await;

            let events: BTreeMap<NaiveDate, Vec<Event>> = BTreeMap::from([(
                original_date.add(TimeDelta::weeks(12)),
                Vec::from([random_event.1]),
            )]);

            let events = filter_new_events_by_thread(
                &api,
                &api.get_guild(*channel_id).await,
                events,
                *channel_id,
            )
            .await;

            let find_result = find_new_unique_event(&link, events);

            assert!(find_result.is_none());
        }

        fn find_new_unique_event(
            link: &str,
            events: BTreeMap<EventsThread, Vec<Event>>,
        ) -> Option<Event> {
            let mut sent_event: Option<Event> = None;

            for (_, events) in events {
                if let Some(event) = events.into_iter().find(|event| event.link == link) {
                    if sent_event.is_some() {
                        panic!("Event was sent twice!");
                    }

                    sent_event = Some(event);
                }
            }

            sent_event
        }
    }

    mod helpers {
        use super::{channel_id, tester_token, token};
        use alertaemcena::agenda_cultural::model::{Event, EventDetails, Schedule};
        use alertaemcena::discord::api::{DiscordAPI, EventsThread};
        use chrono::NaiveDate;
        use lazy_static::lazy_static;
        use serenity::all::{ChannelId, Message};
        use tokio::sync::OnceCell;
        use uuid::Uuid;

        lazy_static! {
            static ref INIT: OnceCell<i32> = OnceCell::new();
        }

        pub async fn send_random_event(api: &DiscordAPI) -> (ChannelId, String, Message) {
            let (link, unique_event, date) = generate_random_event();
            let (thread, message) = send_event(api, unique_event, date).await;

            (thread.thread_id, link, message)
        }

        pub async fn send_event(
            api: &DiscordAPI,
            event: Event,
            date: NaiveDate,
        ) -> (EventsThread, Message) {
            let guild = api.get_guild(*channel_id).await;
            let active_threads = api.get_channel_threads(&guild, *channel_id).await;
            let thread = api
                .get_date_thread(&active_threads, *channel_id, date)
                .await;
            let message = api.send_event(thread.thread_id, event).await;
            (thread, message)
        }

        pub fn generate_random_event() -> (String, Event, NaiveDate) {
            let test_id = Uuid::new_v4();
            let link = format!(
                "https://example.com/events/barca_inferno?test_id={}",
                test_id
            );
            let date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
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
            (link, unique_event, date)
        }

        pub async fn build_api() -> DiscordAPI {
            let api = DiscordAPI::new(&token, true).await;

            let _ = INIT
                .get_or_init(|| async {
                    cleanup_channel(&api).await;
                    0
                })
                .await;

            api
        }

        pub async fn build_api_without_cache() -> DiscordAPI {
            let api = DiscordAPI::new(&token, false).await;

            let _ = INIT
                .get_or_init(|| async {
                    cleanup_channel(&api).await;
                    0
                })
                .await;

            api
        }

        async fn cleanup_channel(api: &DiscordAPI) {
            let guild = &api.get_guild(*channel_id).await;

            for thread in api.get_channel_threads(guild, *channel_id).await {
                thread
                    .delete(&api.client.http)
                    .await
                    .expect("Failed to delete thread!");
            }

            api.delete_all_messages(&channel_id).await;
        }

        pub async fn build_tester_api() -> DiscordAPI {
            DiscordAPI::new(&tester_token, false).await
        }
    }
}
