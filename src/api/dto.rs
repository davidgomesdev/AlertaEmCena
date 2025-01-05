use super::model::{Event, Schedule};
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
    pub string_dates: String,
    pub string_times: String,
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
            occurring_at: Schedule {
                dates: self.string_dates.clone(),
                times: self.string_times.clone()
            },
            venue: self
                .venue
                .first_key_value()
                .map(|venue| venue.1.name.clone())
                .unwrap_or_else(|| "".to_string()),
        }
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
