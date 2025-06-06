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
    async fn should_scrape_the_specified_event_by_public_url() {
        let event: Event = AgendaCulturalAPI::get_event_by_public_url(
            "https://www.agendalx.pt/events/event/nora-helmer/",
        )
        .await
        .unwrap();

        assert_eq!(event.title, "Nora Helmer");
        assert!(event.details.description.starts_with("A história de Nora Helmer, protagonista de Casa de Bonecas, peça de Henrik Ibsen, torna-se o ponto de partida para um debate aceso sobre a família"));
        assert_eq!(event.details.image_url, "https://www.agendalx.pt/content/uploads/2025/02/Nora-Helmer_ensaios2©Filipe_Figueiredo.jpg");
        assert_eq!(
            event.details.subtitle,
            "A partir de Henrik Ibsen e Lucas Hnath"
        );
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/nora-helmer/"
        );
        assert_eq!(event.occurring_at.dates, "8 março a 11 maio");
        assert_eq!(
            event.occurring_at.times,
            "qua: 19h; qui: 19h; sex: 21h; sáb: 21h; dom: 16h"
        );
        assert_eq!(event.venue, "Teatro Aberto");
        assert!(event.tags.is_empty());
    }

    #[test_log::test(tokio::test)]
    async fn should_scrape_the_specified_event_by_id() {
        let event: Event = AgendaCulturalAPI::get_event_by_id(208058).await.unwrap();

        assert_eq!(event.title, "Nora Helmer");
        assert_eq!(event.details.description, "A história de Nora Helmer, protagonista de Casa de Bonecas, peça de Henrik Ibsen, torna-se o ponto de partida para um debate aceso sobre a família, o casamento e o lugar da mulher na sociedade. Ler mais.");
        assert_eq!(event.details.image_url, "https://www.agendalx.pt/content/uploads/2025/02/Nora-Helmer_ensaios2©Filipe_Figueiredo.jpg");
        assert_eq!(
            event.details.subtitle,
            "A partir de Henrik Ibsen e Lucas Hnath"
        );
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/nora-helmer/"
        );
        assert_eq!(event.occurring_at.dates, "8 março a 11 maio");
        assert_eq!(
            event.occurring_at.times,
            "qua: 19h; qui: 19h; sex: 21h; sáb: 21h; dom: 16h"
        );
        assert!(!event.is_for_children);
        assert_eq!(event.venue, "Teatro Aberto");
        assert!(event.tags.is_empty());
    }

    #[test_log::test(tokio::test)]
    async fn should_scrape_the_specified_event_with_an_italic_description_by_public_url() {
        let event: Event = AgendaCulturalAPI::get_event_by_public_url(
            "https://www.agendalx.pt/events/event/king-size-2/",
        )
            .await
            .unwrap();

        assert_eq!(event.title, "King Size");
        assert_eq!(event.details.description, "Através do burlesco dos shows drag king, Sónia Baptista, acompanhada pelas performers Ana Libório, Crista Alfaiate e Joana Levi, desconstrói sem apelo nem agravo, mas com um irresistível humor, os códigos de construção da masculinidade. +");
        assert_eq!(
            event.details.image_url,
            "https://www.agendalx.pt/content/uploads/2025/06/King-Size_MRL5756.jpg"
        );
        assert_eq!(event.details.subtitle, "Sónia Baptista");
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/king-size-2/"
        );
        assert_eq!(event.occurring_at.dates, "6 junho a 15 junho");
        assert_eq!(
            event.occurring_at.times,
            "qui: 21h; sex: 21h; sáb: 19h; dom: 16h"
        );
        assert_eq!(event.venue, "Sala Estúdio Valentim de Barros");
        assert_eq!(event.tags.len(), 4);
        assert!(!event.is_for_children);
        assert_eq!(
            event.tags,
            ["+16", "performance", "queer", "Sónia Baptista"]
        );
    }

    #[test_log::test(tokio::test)]
    async fn should_scrape_and_classify_a_children_piece_as_such() {
        let event: Event = AgendaCulturalAPI::get_event_by_public_url(
            "https://www.agendalx.pt/events/event/um-sapato-especial/",
        )
            .await
            .unwrap();

        assert_eq!(event.title, "Um sapato especial");
        assert_eq!(event.details.description, "O Ursinho José gosta muito de ir brincar para o jardim. Joga à bola, às corridas, anda de bicicleta e nos baloiços. E ele é o campeão dos saltos! Mas um dia acontece algo inesperado e começa uma aventura que lhe trará novos amigos. Com o bombeiro Mário, a Dra. Malaquias e a enfermeira Juju, o Ursinho José vai aprender a divertir-se em segurança.");
        assert_eq!(
            event.details.image_url,
            "https://www.agendalx.pt/content/uploads/2018/09/T-SE-cartaz1.jpg"
        );
        assert_eq!(event.details.subtitle, "");
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/um-sapato-especial/"
        );
        assert_eq!(event.occurring_at.dates, "14 junho");
        assert_eq!(
            event.occurring_at.times,
            "16h00"
        );
        assert_eq!(event.venue, "Fábrica Braço de Prata");
        assert_eq!(event.tags.len(), 2);
        assert_eq!(
            event.tags,
            ["crianças", "famílias"]
        );
        assert!(event.is_for_children);
    }
}
