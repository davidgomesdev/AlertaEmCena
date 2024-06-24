pub mod scraper;

use scraper::scrape_bol;

#[tokio::main]
async fn main() {
  println!("{:?}", scrape_bol().await)
}
