use alertaemcena::agenda_cultural::api::get_events;
use alertaemcena::agenda_cultural::model::Category;

#[tokio::test]
async fn test_teatro_scrape() {
  let res = get_events(5, &Category::Teatro)
      .await
      .unwrap();

  assert_eq!(res.len(), 5);
}

#[tokio::test]
async fn test_artes_scrape() {
  let res = get_events(2, &Category::Artes)
      .await
      .unwrap();

  assert_eq!(res.len(), 2);
}
