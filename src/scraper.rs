use std::error::Error;

use chrono::NaiveDate;
use lazy_static::lazy_static;

use scraper::{selectable::Selectable, ElementRef, Html, Selector};
use tracing::info;

const BOL_BASE_URL: &str = "https://www.bol.pt";

lazy_static! {
    static ref PIECES_URL: String = format!(
        "{}/Comprar/pesquisa/1-101-0-0-0-0/bilhetes_de_teatro_arte_teatro",
        BOL_BASE_URL
    );
    static ref PIECES_SELECTOR: Selector = Selector::parse(".itens").unwrap();
    static ref PIECE_DETAILS_SELECTOR: Selector =
        Selector::parse(".item-montra.evento > .overlay > .nome").unwrap();

    static ref PIECE_IMAGE_SELECTOR: Selector = Selector::parse("#ImagemEvento").unwrap();
    static ref PIECE_DATES_SELECTOR: Selector = Selector::parse(".datas > .sessao").unwrap();
    static ref PIECE_DATE_DAY_SELECTOR: Selector = Selector::parse(".dia").unwrap();
    static ref PIECE_DATE_MONTH_SELECTOR: Selector = Selector::parse(".mes").unwrap();
    static ref PIECE_DATE_YEAR_SELECTOR: Selector = Selector::parse(".ano").unwrap();
    static ref PIECE_PLACE_SELECTOR: Selector = Selector::parse(".detalhes > h4").unwrap();
    static ref PIECE_ADDRESS_SELECTOR: Selector = Selector::parse(".localizacao-entidade > .clearfix > .col-sm-6 > div").unwrap();
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub name: String,
    pub url: String,
    pub thumbnail_url: String,
    pub date_range: (NaiveDate, NaiveDate),
    pub location: PieceLocation
}

#[derive(Debug, Clone)]
pub struct PieceLocation {
    pub city: String,
    pub street: String,
    pub place: String
}

impl Piece {
    pub fn new(
        name: &str,
        piece_path: &str,
        thumbnail_url: &str,
        date_range: (NaiveDate, NaiveDate),
    ) -> Piece {
        Piece {
            name: name.to_string(),
            url: format!("{}{}", BOL_BASE_URL.to_owned(), piece_path),
            thumbnail_url: thumbnail_url.to_string(),
            date_range,
        }
    }
}

pub async fn scrape_bol() -> Result<Vec<Piece>, Box<dyn Error>> {
    let html = crawl_page(PIECES_URL.to_owned()).await?;

    info!("Scraped bol main page");

    let piece_titles = futures::future::join_all(
        html.select(&PIECES_SELECTOR)
            .next()
            .unwrap()
            .select(&PIECE_DETAILS_SELECTOR)
            .map(|el| scrape_piece(el)),
    )
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

    info!("Crawled {}", piece_path);

    let thumbnail_url = piece_html
        .select(&PIECE_IMAGE_SELECTOR)
        .next()
        .unwrap_or_else(|| panic!("No image found on {}", element.html()))
        .attr("src")
        .unwrap_or_else(|| panic!("No image URL found on {}", element.html()));
    let name = element.inner_html().replace('"', "");
    // TODO: fix this failing when the dates are only one day (not a range)
    let dates: Vec<NaiveDate> = piece_html
        .select(&PIECE_DATES_SELECTOR)
        .map(|el| parse_date_element(el)).collect();

    let start_date = dates.first().unwrap_or_else(|| panic!("No dates were found for piece '{}'", name));
    let end_date = match dates.len() {
        1 => start_date,
        2 => dates.last().unwrap(),
        _ => panic!("Dates for piece '{}' has a weird date count: {}", name, dates.len())
    };

    info!("Scraped {}", name);

    Piece::new(&name, piece_path, thumbnail_url, (*start_date, *end_date))
}

async fn crawl_page(url: String) -> Result<Html, Box<dyn Error>> {
    let page_html = reqwest::get(url).await?.text().await?;

    Ok(Html::parse_fragment(&page_html))
}

fn parse_date_element(piece_date: ElementRef) -> NaiveDate {
    let day: u32 = piece_date
        .select(&PIECE_DATE_DAY_SELECTOR)
        .next()
        .unwrap_or_else(|| panic!("Couldn't get day of piece on {}", piece_date.html()))
        .inner_html()
        .parse()
        .unwrap_or_else(|e| {
            panic!(
                "Couldn't parse day of piece on {}, due to {}",
                piece_date.html(),
                e
            )
        });
    let month: u32 = parse_month(
        &piece_date
            .select(&PIECE_DATE_MONTH_SELECTOR)
            .next()
            .unwrap_or_else(|| panic!("Couldn't get month of piece on {}", piece_date.html()))
            .inner_html(),
    );
    let year: i32 = piece_date
        .select(&PIECE_DATE_YEAR_SELECTOR)
        .next()
        .unwrap_or_else(|| panic!("Couldn't get year of piece on {}", piece_date.html()))
        .inner_html()
        .parse()
        .unwrap_or_else(|e| {
            panic!(
                "Couldn't parse year of piece on {}, due to {}",
                piece_date.html(),
                e
            )
        });

    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_else(|| panic!("Failed parsing {}:{}:{} to date", day, month, year))
}

fn parse_month(month_text: &str) -> u32 {
    match month_text {
        "jan" => 1,
        "fev" => 2,
        "mar" => 3,
        "abr" => 4,
        "mai" => 5,
        "jun" => 6,
        "jul" => 7,
        "ago" => 8,
        "set" => 9,
        "out" => 10,
        "nov" => 11,
        "dez" => 12,
        _ => panic!("Got invalid month text to parse '{}'", month_text),
    }
}
