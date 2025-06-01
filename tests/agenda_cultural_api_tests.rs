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
        assert!(event.details.description.starts_with("King Size analisa como se constrói a masculinidade, para ressignificar as relações entre natureza,"));
        assert_eq!(
            event.details.image_url,
            "https://www.agendalx.pt/content/uploads/2025/03/King-Size.jpg"
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
        assert_eq!(event.tags.len(), 2);
        assert_eq!(event.tags.first().unwrap(), "performance");
        assert_eq!(event.tags.get(1).unwrap(), "Sónia Baptista");
    }
}
