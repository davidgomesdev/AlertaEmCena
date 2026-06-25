mod discord_embed_color {
    use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
    use alertaemcena::discord::api::DiscordAPI;
    use serenity::all::Colour;

    const FALLBACK_COLOR: Colour = Colour::new(0x005eeb);

    async fn assert_dominant_color_extracted(event_url: &str) {
        let event = AgendaCulturalAPI::scrape_event(event_url)
            .await
            .unwrap_or_else(|| panic!("Failed to scrape event '{}'", event_url));

        let color = DiscordAPI::get_image_dominant_color(&event.details.image_url).await;

        println!("Got {:?} for event '{}'", color.unwrap().hex(), event.details.image_url);

        match color {
            Some(color) => assert_ne!(
                color, FALLBACK_COLOR,
                "Dominant color for '{}' matched the fallback color by coincidence or extraction failed silently",
                event.details.image_url
            ),
            None => panic!(
                "Failed to extract dominant color from event image '{}'",
                event.details.image_url
            ),
        }
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_sonho_de_uma_noite_de_verao() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/sonho-de-uma-noite-de-verao-5/",
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_o_filho() {
        assert_dominant_color_extracted("https://www.agendalx.pt/events/event/o-filho-2/").await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_o_coracao_de_um_pugilista() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/o-coracao-de-um-pugilista/",
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_a_ratoeira() {
        assert_dominant_color_extracted("https://www.agendalx.pt/events/event/a-ratoeira-5/")
            .await;
    }
}
