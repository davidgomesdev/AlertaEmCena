use super::model::{Event, EventDetails, Schedule};
use futures::TryFutureExt;
use log::warn;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_either::SingleOrVec;
use std::collections::BTreeMap;
use voca_rs::strip::strip_tags;

#[derive(Deserialize)]
pub struct ResponseEvent {
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

        Event::new(
            self.title.rendered.to_string(),
            EventDetails::new(subtitle, description, self.featured_media_large.to_string()),
            self.link.to_string(),
            Schedule::new(self.string_dates.to_string(), self.string_times.to_string()),
            self.venue
                .first_key_value()
                .map(|venue| venue.1.name.to_string())
                .unwrap_or_default(),
        )
    }

    async fn crawl_full_description(&self) -> String {
        let full_page: Result<String, _> = reqwest::get(&self.link)
            .inspect_err(|err| warn!("Failed to get full page {}: {}", self.link, err))
            .and_then(|res| {
                res.text()
                    .inspect_err(|err| warn!("Failed to get full page text {}: {}", self.link, err))
            })
            .await;

        if full_page.is_err() {
            warn!("Using only preview description");
            return self.description.concat().to_string()
        }

        let page_html = Html::parse_fragment(full_page.unwrap().as_str());
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
                warn!(
                    "Not able to find description in page {} (Half description is {})",
                    self.link, half_description
                );
                &preview_description
            })
            .to_string();

        strip_tags(&description)
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
