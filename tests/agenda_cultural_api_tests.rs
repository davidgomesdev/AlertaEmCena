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
        assert!(event.details.description.starts_with("A história de Nora Helmer, protagonista de Casa de Bonecas, peça de Henrik Ibsen"));
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
            "https://www.agendalx.pt/events/event/o-monte/",
        )
            .await
            .unwrap();

        assert_eq!(event.title, "O Monte");
        assert!(event.details.description.starts_with("A partir de um texto de João Ascenso, O Monte é inspirado no relato da atriz Luísa Ortigoso sobre um ex-preso político que reencontra o seu torturador anos após a ditadura."));
        assert_eq!(
            event.details.image_url,
            "https://www.agendalx.pt/content/uploads/2025/03/omonte.jpg"
        );
        assert_eq!(event.details.subtitle, "Teatro Livre");
        assert_eq!(
            event.link,
            "https://www.agendalx.pt/events/event/o-monte/"
        );
        assert_eq!(event.occurring_at.dates, "24 abril a 4 maio");
        assert_eq!(
            event.occurring_at.times,
            "qua: 21h30; qui: 21h30; sex: 21h30; sáb: 18h; dom: 18h"
        );
        assert_eq!(event.venue, "Teatro do Bairro");
        assert_eq!(event.tags.len(), 3);
        assert!(!event.is_for_children);
        assert_eq!(
            event.tags,
            ["Cucha Carvalheiro", "Miguel Sopas", "Teatro Livre"]
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
        assert!(event.details.description.starts_with("O Ursinho José gosta muito de ir brincar para o jardim. Joga à bola, às corridas, anda de bicicleta e nos baloiços. E ele é o campeão dos saltos! Mas um dia acontece algo inesperado e começa um"));
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
