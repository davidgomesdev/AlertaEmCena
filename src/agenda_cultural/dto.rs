use super::model::{Event, EventDetails, Schedule};
use futures::TryFutureExt;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Response;
use scraper::{Html, Selector};
use serde::{de, Deserialize, Deserializer};
use serde_either::SingleOrVec;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use tracing::warn;
use voca_rs::strip::strip_tags;

#[derive(Debug, Deserialize)]
pub struct ResponseEvent {
    pub title: ResponseTitle,
    pub subtitle: SingleOrVec<String>,
    pub description: Vec<String>,
    #[serde(deserialize_with = "deserialize_str")]
    pub featured_media_large: String,
    #[serde(deserialize_with = "deserialize_str")]
    pub link: String,
    pub string_dates: String,
    pub string_times: String,
    #[serde(deserialize_with = "deserialize_btreemap")]
    pub venue: BTreeMap<String, ResponseVenue>,
    #[serde(deserialize_with = "deserialize_btreemap", rename = "tags_name_list")]
    pub tags: BTreeMap<String, ResponseEventTag>,
}

lazy_static! {
    static ref REMOVE_YEAR: Regex = Regex::new(r" *?(\d{4}) *?").unwrap();
}

impl ResponseEvent {
    #[tracing::instrument(skip(self), fields(self.link = %self.link))]
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
            Schedule::new(
                Self::get_date_description(&self.string_dates),
                self.string_times.to_string(),
            ),
            self.venue
                .iter()
                .find(|(_, venue)| !venue.name.is_empty())
                .map(|venue| venue.1.name.to_string())
                .unwrap_or_else(|| {
                    warn!("No venue name found (omitting venue)");
                    "".to_string()
                }),
            self.tags.iter().map(|dto| dto.1.name.to_string()).collect(),
        )
    }

    fn get_date_description(schedule_dates: &str) -> String {
        let years = REMOVE_YEAR
            .captures_iter(schedule_dates)
            .map(|a| a[1].to_string())
            .collect::<Vec<String>>();

        let year_count = years.len();
        if year_count >= 2 {
            let unique_years = years.iter().collect::<HashSet<&String>>();

            if unique_years.len() == 1 {
                Self::remove_year_from_description(schedule_dates)
            } else {
                schedule_dates.to_string()
            }
        } else {
            Self::remove_year_from_description(schedule_dates)
        }
    }

    fn remove_year_from_description(date: &str) -> String {
        REMOVE_YEAR.replace_all(date, "").to_string()
    }

    async fn crawl_full_description(&self) -> String {
        let full_page: Result<String, _> = reqwest::get(&self.link)
            .inspect_err(|err| warn!("Failed to get full page: {:?}", err))
            .and_then(|res: Response| {
                res.text()
                    .inspect_err(|err| warn!("Failed to get full page text: {}", err))
            })
            .await;

        if full_page.is_err() {
            warn!("Using only preview description");
            return strip_tags(&self.description.concat());
        }

        let page_html = Html::parse_fragment(full_page.unwrap().as_str());
        let description = page_html
            .select(&Selector::parse(".entry-container > p").unwrap())
            .map(|p| p.inner_html().to_string())
            .collect::<Vec<String>>()
            .first()
            .map(|v| v.to_owned())
            .unwrap_or_else(|| {
                let preview_description = self.description.concat();
                warn!(
                    "Not able to find description in page (using preview description: {})",
                    preview_description
                );
                preview_description
            });

        strip_tags(&description)
    }
}

#[derive(Debug, Deserialize)]
pub struct ResponseTitle {
    pub rendered: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseVenue {
    #[serde(deserialize_with = "deserialize_str")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseEventTag {
    #[serde(deserialize_with = "deserialize_str")]
    pub name: String,
}

fn deserialize_btreemap<'de, D, T>(d: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(BTreeMap::deserialize(d).unwrap_or(BTreeMap::new()))
}

fn deserialize_str<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Value::deserialize(d)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_a_date_spans_only_one_year_should_get_only_day_and_month() {
        let result = ResponseEvent::get_date_description("28 janeiro a 18 novembro 2025");

        assert_eq!(result, "28 janeiro a 18 novembro");
    }

    #[test_log::test]
    fn when_a_date_spans_two_equal_years_should_get_both_day_month_and_year() {
        let result = ResponseEvent::get_date_description("2 março 2025 a 1 setembro 2025");

        assert_eq!(result, "2 março a 1 setembro");
    }

    #[test_log::test]
    fn when_a_date_spans_two_different_years_should_get_both_day_month_and_year() {
        let result = ResponseEvent::get_date_description("2 novembro 2024 a 1 junho 2025");

        assert_eq!(result, "2 novembro 2024 a 1 junho 2025");
    }

    #[test]
    fn when_a_date_has_only_one_day_should_get_day_and_month() {
        let result = ResponseEvent::get_date_description("3 maio 2025");

        assert_eq!(result, "3 maio");
    }
}
