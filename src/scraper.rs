use std::error::Error;

use lazy_static::lazy_static;

use scraper::{Html, Selector};

const BOL_BASE_URL: &str = "https://www.bol.pt";

lazy_static! {
    static ref PIECES_URL: String = format!(
        "{}/Comprar/pesquisa/1-101-0-0-0-0/bilhetes_de_teatro_arte_teatro",
        BOL_BASE_URL
    );
}

const PIECES_SELECTOR: &str = ".item-montra > .overlay > .nome";

#[derive(Debug, Clone)]
pub struct Piece {
    pub name: String,
    pub url: String,
}

impl Piece {
    pub fn new(name: &str, url: &str) -> Piece {
        Piece {
            name: name.to_string(),
            url: format!("{}{}", BOL_BASE_URL.to_owned(), url.to_string()),
        }
    }
}

pub async fn scrape_bol() -> Result<Vec<Piece>, Box<dyn Error>> {
    let html = crawl_bol_pieces_page().await?;
    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse(PIECES_SELECTOR)?;

    let piece_titles = fragment
        .select(&selector)
        .map(|el| {
            Piece::new(
                &el.inner_html().replace("\"", ""),
                el.attr("href")
                    .expect(&format!("Got invalid link on {}", el.html())),
            )
        })
        .collect::<Vec<Piece>>();

    Ok(piece_titles)
}

async fn crawl_bol_pieces_page() -> Result<String, Box<dyn Error>> {
    let page_html = reqwest::get(PIECES_URL.to_owned()).await?.text().await?;

    Ok(page_html)
}
