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

#[test_log::test(tokio::test)]
async fn should_scrape_the_specified_event_with_an_italic_description_by_public_url() {
    let event: Event = AgendaCulturalAPI::get_event_by_public_url(
        "https://www.agendalx.pt/events/event/king-size-2/",
    )
    .await
    .unwrap();

    assert_eq!(event.title, "King Size");
    assert_eq!(event.details.description, "King Size analisa como se constrói a masculinidade, para ressignificar as relações entre natureza, género e sexo. Focando figuras que incorporam os mitos masculinos, bem como as suas revisões paródicas, esta criação encena um jogo de desidentificação que resiste às pressões de binarismo sexual, na sociedade e na arte.\n\n.\n\nUma obra que confronta os dispositivos de criação e dramaturgia de performances drag contemporâneas com os códigos rígidos de representação de género, na dança e no teatro tradicionais, e interroga essas representações, esbatendo as diferenças entre o que é natural e o que é construção cultural ou cénica.");
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
    assert_eq!(event.tags.get(0).unwrap(), "performance");
    assert_eq!(event.tags.get(1).unwrap(), "Sónia Baptista");
}
