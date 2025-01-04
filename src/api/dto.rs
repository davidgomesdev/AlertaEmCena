use std::{fmt, marker::PhantomData, str::FromStr};

use serde::{de::Visitor, Deserialize, Deserializer};
use serde_either::SingleOrVec;

use super::model::Event;

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
            start_dates: self.start_dates.clone(),
            last_dates: self.last_dates.clone(),
        }
    }
}

#[derive(Deserialize)]
pub struct ResponseTitle {
    pub rendered: String,
}
