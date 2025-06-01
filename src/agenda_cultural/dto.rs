use super::model::{Event, EventDetails, Schedule};
use chrono::NaiveDate;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use serde_either::SingleOrVec;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct SingleEventResponse {
    #[serde(rename = "data")]
    pub event: EventResponse,
}

// Note: some String fields need the custom deserializer due to being optional
#[derive(Debug, Deserialize)]
pub struct EventResponse {
    pub title: ResponseTitle,
    pub subtitle: SingleOrVec<String>,
    pub description: Vec<String>,
    #[serde(deserialize_with = "deserialize_str")]
    pub featured_media_large: String,
    #[serde(deserialize_with = "deserialize_str")]
    pub link: String,
    pub string_dates: String,
    pub string_times: String,
    #[serde(rename = "StartDate", deserialize_with = "deserialize_date")]
    pub start_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_btreemap")]
    pub venue: BTreeMap<String, ResponseVenue>,
    #[serde(deserialize_with = "deserialize_btreemap", rename = "tags_name_list")]
    pub tags: BTreeMap<String, ResponseEventTag>,
}

lazy_static! {
    static ref REMOVE_YEAR: Regex = Regex::new(r" *?(\d{4}) *?").unwrap();
}

impl EventResponse {
    #[tracing::instrument(skip(self), fields(self.link = %self.link))]
    pub async fn to_model(&self, description: String) -> Event {
        let subtitle = match self.subtitle.clone() {
            SingleOrVec::Single(subtitle) => subtitle,
            SingleOrVec::Vec(vec) => vec.concat(),
        };

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
    let value = Value::deserialize(d)?;
    Ok(match value {
        Value::Object(_) => BTreeMap::deserialize(value).unwrap_or(BTreeMap::new()),
        _ => BTreeMap::new(),
    })
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

fn deserialize_date<'de, D>(d: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(d)? {
        Value::String(s) => {
            if s.is_empty() {
                return Ok(NaiveDate::MIN);
            }

            Ok(
                NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap_or_else(|err| {
                    warn!("Failed to parse date. Err: {err}");
                    NaiveDate::MIN
                }),
            )
        }
        _unknown => panic!("Found an unknown data type: {}", _unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn when_a_date_spans_only_one_year_should_get_only_day_and_month() {
        let result = EventResponse::get_date_description("28 janeiro a 18 novembro 2025");

        assert_eq!(result, "28 janeiro a 18 novembro");
    }

    #[test_log::test]
    fn when_a_date_spans_two_equal_years_should_get_both_day_month_and_year() {
        let result = EventResponse::get_date_description("2 março 2025 a 1 setembro 2025");

        assert_eq!(result, "2 março a 1 setembro");
    }

    #[test_log::test]
    fn when_a_date_spans_two_different_years_should_get_both_day_month_and_year() {
        let result = EventResponse::get_date_description("2 novembro 2024 a 1 junho 2025");

        assert_eq!(result, "2 novembro 2024 a 1 junho 2025");
    }

    #[test_log::test]
    fn when_a_date_has_only_one_day_should_get_day_and_month() {
        let result = EventResponse::get_date_description("3 maio 2025");

        assert_eq!(result, "3 maio");
    }

    #[test_log::test]
    fn should_deserialize_event_without_tags() {
        let dto = serde_json::from_str::<Vec<EventResponse>>(
            r##"
              [{
                "id": 206968,
                "type": "event",
                "title": {
                  "rendered": "Galafoice"
                },
                "featured_media_large": "https:\/\/www.agendalx.pt\/content\/uploads\/2025\/01\/galafoice.jpg",
                "subtitle": [
                  "Jo\u00e3o Moreira"
                ],
                "subject": "teatro",
                "string_dates": "22 fevereiro a 23 fevereiro 2025",
                "string_times": "s\u00e1b: 21h; dom: 17h",
                "description": [
                  "Espet\u00e1culo inaugural de uma trilogia autobiogr\u00e1fica e autoficcional de <span data-olk-copy-source=\"MessageBody\">Jo\u00e3o Moreira<\/span>. A pe\u00e7a \"funciona ao mesmo tempo como <em>recap<\/em> do passado..."
                ],
                "venue": {
                  "teatro-iberico-2": {
                    "id": 328,
                    "slug": "teatro-iberico-2",
                    "name": "Teatro Ib\u00e9rico"
                  }
                },
                "categories_name_list": {
                  "teatro": {
                    "id": 43,
                    "slug": "teatro",
                    "name": "teatro"
                  }
                },
                "tags_name_list": [],
                "link": "https:\/\/www.agendalx.pt\/events\/event\/galafoice\/",
                "occurences": [
                  "2025-02-22",
                  "2025-02-23"
                ],
                "StartDate": "2025-02-22",
                "LastDate": "2025-02-23",
                "price_cat": [
                  "unknown"
                ],
                "price_val": "",
                "target_audience": [],
                "accessibility": []
              }]"##,
        );

        assert!(dto.is_ok(), "{:?}", dto);

        let dto = dto.unwrap();

        assert_eq!(dto.len(), 1);

        let dto = dto.first().unwrap();

        assert_eq!(
            dto.start_date,
            NaiveDate::from_ymd_opt(2025, 2, 22).unwrap(),
            "{:?}",
            dto
        );
    }

    #[test_log::test]
    fn should_deserialize_event_with_tags() {
        let dto = serde_json::from_str::<Vec<EventResponse>>(
            r##"
              [{
                "id": 206968,
                "type": "event",
                "title": {
                  "rendered": "Galafoice"
                },
                "featured_media_large": "https:\/\/www.agendalx.pt\/content\/uploads\/2025\/01\/galafoice.jpg",
                "subtitle": [
                  "Jo\u00e3o Moreira"
                ],
                "subject": "teatro",
                "string_dates": "22 fevereiro a 23 fevereiro 2025",
                "string_times": "s\u00e1b: 21h; dom: 17h",
                "description": [
                  "Espet\u00e1culo inaugural de uma trilogia autobiogr\u00e1fica e autoficcional de <span data-olk-copy-source=\"MessageBody\">Jo\u00e3o Moreira<\/span>. A pe\u00e7a \"funciona ao mesmo tempo como <em>recap<\/em> do passado..."
                ],
                "venue": {
                  "teatro-iberico-2": {
                    "id": 328,
                    "slug": "teatro-iberico-2",
                    "name": "Teatro Ib\u00e9rico"
                  }
                },
                "categories_name_list": {
                  "teatro": {
                    "id": 43,
                    "slug": "teatro",
                    "name": "teatro"
                  }
                },
                "tags_name_list": {
                  "gratuito": {
                    "id": 5121,
                    "slug": "gratuito",
                    "name": "gratuito"
                  }
                },
                "link": "https:\/\/www.agendalx.pt\/events\/event\/galafoice\/",
                "occurences": [
                  "2025-02-22",
                  "2025-02-23"
                ],
                "StartDate": "2025-02-22",
                "LastDate": "2025-02-23",
                "price_cat": [
                  "unknown"
                ],
                "price_val": "",
                "target_audience": [],
                "accessibility": []
              }]"##,
        );

        assert!(dto.is_ok(), "{:?}", dto);

        let dto = dto.unwrap();

        assert_eq!(dto.len(), 1);

        let dto = dto.first().unwrap();

        assert_eq!(
            dto.start_date,
            NaiveDate::from_ymd_opt(2025, 2, 22).unwrap(),
            "{:?}",
            dto
        );
    }
}
