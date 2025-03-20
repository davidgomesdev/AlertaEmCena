use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};

#[test_log::test(tokio::test)]
async fn should_scrape_teatro_events() {
    let res = AgendaCulturalAPI::get_events(&Category::Teatro, Some(5))
        .await
        .unwrap();

    assert_eq!(res.len(), 5);
}

#[test_log::test(tokio::test)]
async fn when_no_event_limit_is_provided_should_scrape_more_teatro_events_than_the_default() {
    let res = AgendaCulturalAPI::get_events(&Category::Teatro, None)
        .await
        .unwrap();

    assert_ne!(res.len(), 0);
    // 5 is the default
    // I'd be depressed if there were no more than 5 pieces in the upcoming dates 🥲
    assert!(res.len() > 5);
}

#[test_log::test(tokio::test)]
async fn should_scrape_artes_events() {
    let res = AgendaCulturalAPI::get_events(&Category::Artes, Some(2))
        .await
        .unwrap();

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
    assert_eq!(event.occurring_at.dates, "8 março a 20 abril");
    assert_eq!(
        event.occurring_at.times,
        "qua: 19h; qui: 19h; sex: 21h; sáb: 21h; dom: 16h"
    );
    assert_eq!(event.venue, "Teatro Aberto");
    assert_eq!(event.tags.is_empty(), true);
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
    assert_eq!(event.occurring_at.dates, "8 março a 20 abril");
    assert_eq!(
        event.occurring_at.times,
        "qua: 19h; qui: 19h; sex: 21h; sáb: 21h; dom: 16h"
    );
    assert_eq!(event.venue, "Teatro Aberto");
    assert_eq!(event.tags.is_empty(), true);
}
