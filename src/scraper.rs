use std::error::Error;

use scraper::{Html, Selector};

const PIECES_URL: &str =
    "https://www.bol.pt/Comprar/pesquisa/1-101-0-0-0-0/bilhetes_de_teatro_arte_teatro";

const PIECES_SELECTOR: &str = ".item-montra > .overlay > .nome";

pub async fn scrape_bol() -> Result<Vec<String>, Box<dyn Error>> {
  let html = crawl_bol_pieces_page().await?;
  let fragment = Html::parse_fragment(&html);
  let selector = Selector::parse(PIECES_SELECTOR)?;

  let piece_titles = fragment.select(&selector).map(|el| el.inner_html()).collect::<Vec<String>>();
  
  Ok(piece_titles)
}

async fn crawl_bol_pieces_page() -> Result<String, Box<dyn Error>> {
  let page_html = reqwest::get(PIECES_URL).await?.text().await?;

  Ok(page_html)
}
