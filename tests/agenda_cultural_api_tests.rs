mod agenda_cultural {
    use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
    use alertaemcena::agenda_cultural::model::{Category, Event};

    // ignored since 2026-08-04: the agendalx events endpoint answers `[]` to every query (after
    // taking ~60s), so these can't tell a broken scrape from a broken site. Run them with
    // `cargo test -- --ignored` to check whether it recovered.
    #[ignore]
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

    // see the note on `should_scrape_teatro_events`
    #[ignore]
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
        let event = AgendaCulturalAPI::scrape_event(
            "https://www.agendalx.pt/events/event/mowgli-o-livro-da-selva/",
        )
        .await
        .expect("Failed to scrape event");

        assert_eq!(event.title, "Mowgli – O Livro da Selva");
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/mowgli-o-livro-da-selva/"
        );
        assert_eq!(event.venue, "Teatro Armando Cortez");
        assert_eq!(event.occurring_at.dates, "24 outubro 2026 a 30 maio 2027");
        assert!(
            event.details.description.starts_with("A selva de Rudyard Kipling ganha uma nova vida em palco com Mowgli – O Livro da Selva"),
            "Unexpected description: {}",
            event.details.description
        );
        assert_eq!(
            event.details.image_url,
            "https://www.agendalx.pt/content/uploads/2026/09/foto-promo_mowgli_DSCF9921-copy-scaled.jpg"
        );
    }
}
