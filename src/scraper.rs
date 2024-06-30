use std::error::Error;

use lazy_static::lazy_static;

use scraper::{ElementRef, Html, Selector};

const BOL_BASE_URL: &str = "https://www.bol.pt";

lazy_static! {
    static ref PIECES_URL: String = format!(
        "{}/Comprar/pesquisa/1-101-0-0-0-0/bilhetes_de_teatro_arte_teatro",
        BOL_BASE_URL
    );
    static ref PIECE_ITEMS_SELECTOR: Selector = Selector::parse(".itens").unwrap();
    static ref PIECE_NAME_SELECTOR: Selector =
        Selector::parse(".item-montra.evento > .overlay > .nome").unwrap();
    static ref PIECE_IMAGE_SELECTOR: Selector = Selector::parse("#ImagemEvento").unwrap();
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub name: String,
    pub url: String,
    pub thumbnail_url: String,
}

impl Piece {
    pub fn new(name: &str, piece_path: &str, thumbnail_url: &str) -> Piece {
        Piece {
            name: name.to_string(),
            url: format!("{}{}", BOL_BASE_URL.to_owned(), piece_path),
            thumbnail_url: thumbnail_url.to_string(),
        }
    }
}

pub async fn scrape_bol() -> Result<Vec<Piece>, Box<dyn Error>> {
    let html = crawl_page(PIECES_URL.to_owned()).await?;

    let piece_titles = futures::future::join_all(html
        .select(&PIECE_ITEMS_SELECTOR).next().unwrap()
        .select(&PIECE_NAME_SELECTOR)
        .map(|el| scrape_piece(el)))
        .await;

    Ok(piece_titles)
}

async fn scrape_piece(element: ElementRef<'_>) -> Piece {
    let piece_path = element
        .attr("href")
        .unwrap_or_else(|| panic!("Got invalid link on {}", element.html()));

    let piece_html = crawl_page(format!("{}{}", BOL_BASE_URL.to_owned(), piece_path))
                .await
                .expect("fix");

    let thumbnail_url = piece_html
        .select(&PIECE_IMAGE_SELECTOR)
        .next()
        .unwrap_or_else(|| panic!("No image found on {}", element.html()))
        .attr("src")
        .unwrap_or_else(|| panic!("No image URL found on {}", element.html()));

    Piece::new(
        &element.inner_html().replace('"', ""),
        piece_path,
        thumbnail_url,
    )
}

async fn crawl_page(url: String) -> Result<Html, Box<dyn Error>> {
    let page_html = reqwest::get(url).await?.text().await?;

    Ok(Html::parse_fragment(&page_html))
}
