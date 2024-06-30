pub mod scraper;

use scraper::scrape_bol;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  println!("{:?}", scrape_bol().await.unwrap())
}
