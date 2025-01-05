use super::model::{Event, EventDetails, Schedule};
use log::warn;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_either::SingleOrVec;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct ResponseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: ResponseTitle,
    pub subtitle: SingleOrVec<String>,
    pub description: Vec<String>,
    pub featured_media_large: String,
    pub link: String,
    pub string_dates: String,
    pub string_times: String,
    pub venue: BTreeMap<String, Venue>,
}

impl ResponseEvent {
    pub async fn to_model(&self) -> Event {
        let subtitle = match self.subtitle.clone() {
            SingleOrVec::Single(subtitle) => subtitle,
            SingleOrVec::Vec(vec) => vec.concat(),
        };
        let description = self.crawl_full_description().await;

        Event {
            event_type: self.event_type.to_string(),
            title: self.title.rendered.to_string(),
            details: EventDetails {
                subtitle,
                description,
                image_url: self.featured_media_large.to_string(),
            },
            link: self.link.to_string(),
            occurring_at: Schedule {
                dates: self.string_dates.to_string(),
                times: self.string_times.to_string(),
            },
            venue: self
                .venue
                .first_key_value()
                .map(|venue| venue.1.name.to_string())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    async fn crawl_full_description(&self) -> String {
        let full_page = reqwest::get(&self.link)
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let page_html = Html::parse_fragment(full_page.as_str());
        let preview_description = self.description.concat();
        let half_description = preview_description
            .split_at(preview_description.len() / 2)
            .0;

        let description = page_html
            .select(&Selector::parse("p").unwrap())
            .filter(|p| p.inner_html().starts_with(half_description))
            .map(|p| p.inner_html().to_string())
            .collect::<Vec<String>>()
            .first()
            .unwrap_or_else(|| {
                warn!("Not able to find description in page {}", self.link);
                &preview_description
            })
            .to_string();

        description
    }
}

#[derive(Deserialize)]
pub struct ResponseTitle {
    pub rendered: String,
}

#[derive(Debug, Deserialize)]
pub struct Venue {
    pub name: String,
}
