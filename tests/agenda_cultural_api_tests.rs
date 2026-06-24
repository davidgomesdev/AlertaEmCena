mod agenda_cultural {
    use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
    use alertaemcena::agenda_cultural::model::{Category, Event};

    #[test_log::test(tokio::test)]
    async fn should_scrape_teatro_events() {
        let res: Vec<Event> = AgendaCulturalAPI::get_events_by_month(&Category::Teatro, Some(5))
            .await
            .unwrap()
            .into_values()
            .flatten()
            .collect();

        assert_eq!(res.len(), 5);
    }

    #[test_log::test(tokio::test)]
    async fn should_scrape_artes_events() {
        let res: Vec<Event> = AgendaCulturalAPI::get_events_by_month(&Category::Artes, Some(2))
            .await
            .unwrap()
            .into_values()
            .flatten()
            .collect();

        assert_eq!(res.len(), 2);
    }

    #[test_log::test(tokio::test)]
    async fn should_scrape_single_event_page() {
        let event = AgendaCulturalAPI::scrape_event("https://www.agendalx.pt/events/event/maes/")
            .await
            .expect("Failed to scrape event");

        assert_eq!(event.title, "Mães");
        assert_eq!(event.link, "https://www.agendalx.pt/events/event/maes/");
        assert_eq!(event.venue, "Teatro Villaret");
        assert_eq!(event.occurring_at.dates, "14 março a 30 junho 2024");
        assert_eq!(event.details.description, "Três mães e uma grávida juntas num musical hilariante e ternurento onde ficamos a conhecer a poderosa amizade de quatro mulheres…");
        assert_eq!(event.details.image_url, "https://www.agendalx.pt/content/uploads/2024/02/Maes.jpg");
    }
}
