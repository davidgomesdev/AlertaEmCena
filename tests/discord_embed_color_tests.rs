mod discord_embed_color {
    use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
    use alertaemcena::discord::api::DiscordAPI;
    use serenity::all::Colour;

    async fn assert_dominant_color_extracted(event_url: &str, expected_hex: u32) {
        let event = AgendaCulturalAPI::scrape_event(event_url)
            .await
            .unwrap_or_else(|| panic!("Failed to scrape event '{}'", event_url));

        let color = DiscordAPI::get_image_dominant_color(&event.details.image_url).await;

        match color {
            Some(color) => assert_eq!(
                color,
                Colour::new(expected_hex),
                "Dominant color for '{}' did not match expected",
                event.details.image_url
            ),
            None => panic!(
                "Failed to extract dominant color from event image '{}'",
                event.details.image_url
            ),
        }
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_mil_vezes_revista() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/mil-vezes-revista/",
            0xDF9C6D,
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_clube_dos_poetas_mortos() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/clube-dos-poetas-mortos/",
            0xB99E93,
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_cock() {
        assert_dominant_color_extracted("https://www.agendalx.pt/events/event/cock/", 0x2B2B2B)
            .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_branca_de_neve() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/branca-de-neve/",
            0xBE1A18,
        )
        .await;
    }
}
