use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::Category;

#[tokio::test]
#[test_log::test]
async fn should_scrape_teatro_events() {
  let res = AgendaCulturalAPI::get_events(5, &Category::Teatro)
      .await
      .unwrap();

  assert_eq!(res.len(), 5);
}

#[tokio::test]
#[test_log::test]
async fn should_scrape_artes_events() {
  let res = AgendaCulturalAPI::get_events(2, &Category::Artes)
      .await
      .unwrap();

  assert_eq!(res.len(), 2);
}
