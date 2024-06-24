use alertaemcena::scraper;

#[tokio::test]
async fn test_scrape() {
  let res = scraper::scrape_bol().await.unwrap();

  assert_ne!(res.len(), 0);
}
