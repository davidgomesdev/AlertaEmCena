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
    async fn should_extract_dominant_color_for_sonho_de_uma_noite_de_verao() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/sonho-de-uma-noite-de-verao-5/",
            0x1B80B4,
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_o_filho() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/o-filho-2/",
            0x785D55,
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_o_coracao_de_um_pugilista() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/o-coracao-de-um-pugilista/",
            0xCEB2A7,
        )
        .await;
    }

    #[test_log::test(tokio::test)]
    async fn should_extract_dominant_color_for_a_ratoeira() {
        assert_dominant_color_extracted(
            "https://www.agendalx.pt/events/event/a-ratoeira-5/",
            0xC07A4F,
        )
        .await;
    }
}
