use super::model::{DateRange, Event};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_either::SingleOrVec;
use std::collections::BTreeMap;

const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ResponseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: ResponseTitle,
    pub subtitle: SingleOrVec<String>,
    pub description: Vec<String>,
    pub link: String,
    #[serde(rename = "StartDate")]
    pub start_dates: String,
    #[serde(rename = "LastDate")]
    pub last_dates: String,
    pub venue: BTreeMap<String, Venue>,
}

impl ResponseEvent {
    pub fn to_model(&self) -> Event {
        let subtitle = match self.subtitle.clone() {
            SingleOrVec::Single(subtitle) => subtitle,
            SingleOrVec::Vec(vec) => vec.concat(),
        };

        Event {
            event_type: self.event_type.clone(),
            title: self.title.rendered.clone(),
            subtitle,
            description: self.description.concat(),
            link: self.link.clone(),
            occurring_at: DateRange {
                start: Self::parse_date(&self.start_dates),
                end: Self::parse_date(&self.last_dates),
            },
            venue: self
                .venue
                .first_key_value()
                .map(|venue| venue.1.name.clone())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    fn parse_date(date: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(date, &DATE_FORMAT)
            .map(|res| Some(res))
            .unwrap_or(None)
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
