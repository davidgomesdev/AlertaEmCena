use super::{dto::EventResponse, model::Event};
use crate::agenda_cultural::dto::SingleEventResponse;
use crate::agenda_cultural::model::Category;
use chrono::{Datelike, NaiveDate};
use futures::TryFutureExt;
use lazy_static::lazy_static;
use reqwest::{Client, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use scraper::{Html, Selector};
use std::collections::BTreeMap;
use tracing::{error, info, warn};

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";
const AGENDA_PAGE_BY_ID_PATH: &str = "https://www.agendalx.pt/?p=";
const EVENT_TYPE: &str = "event";
const MAX_RETRIES: u32 = 5;

lazy_static! {
    static ref REST_CLIENT: ClientWithMiddleware = ClientBuilder::new(Client::new())
        .with(RetryTransientMiddleware::new_with_policy(
            ExponentialBackoff::builder().build_with_max_retries(MAX_RETRIES)
        ))
        .build();
    static ref EVENT_ID_SELECTOR: Selector = Selector::parse(&format!(
        r#"link[rel="shortlink"][href^="{}"]"#,
        AGENDA_PAGE_BY_ID_PATH
    ))
    .unwrap();
}

pub struct AgendaCulturalAPI;

impl AgendaCulturalAPI {
    /**
    Returns events in ascending order
    * amount_per_page: if not specified, will retrieve everything
    */
    #[tracing::instrument]
    pub async fn get_events_by_month(
        category: &Category,
        amount_per_page: Option<i32>,
    ) -> Result<BTreeMap<NaiveDate, Vec<Event>>, APIError> {
        match amount_per_page {
            None => {
                info!("Getting all events");
            }
            Some(amount) => {
                info!("Getting {} events", amount);
            }
        }

        let category: &'static str = category.into();
        let parsed_response = Self::get_events_by_category(amount_per_page, category).await;

        match parsed_response {
            Ok(parsed_response) => {
                info!("Fetched {} events", parsed_response.len());

                let events = Self::parse_events_by_date(parsed_response).await;

                Ok(events)
            }
            Err(e) => {
                error!("Response parse failed: {:?}", e);
                Err(APIError::InvalidResponse)
            }
        }
    }

    async fn parse_events_by_date(response: Vec<EventResponse>) -> BTreeMap<NaiveDate, Vec<Event>> {
        let mut events_by_date: BTreeMap<NaiveDate, Vec<Event>> = BTreeMap::new();

        for response in response {
            let model = response.to_model().await;
            let date =
                NaiveDate::from_ymd_opt(response.start_date.year(), response.start_date.month(), 1)
                    .unwrap();

            if let Some(events) = events_by_date.get_mut(&date) {
                events.push(model);
            } else {
                events_by_date.insert(date, Vec::from([model]));
            }
        }

        events_by_date
    }

    /**
    Returns the specified event
    */
    #[tracing::instrument]
    pub async fn get_event_by_id(event_id: u32) -> Result<Event, APIError> {
        let json_response = REST_CLIENT
            .get(format!("{}/{}", AGENDA_EVENTS_URL, event_id))
            .send()
            .await
            .expect("Error sending request")
            .error_for_status()
            .expect("Request failed")
            .text()
            .await
            .expect("Received invalid response");
        let parsed_response = serde_json::from_str::<SingleEventResponse>(&json_response);

        match parsed_response {
            Ok(dto) => Ok(dto.event.to_model().await),
            Err(e) => {
                error!("Response parse failed: {:?}", e);
                Err(APIError::InvalidResponse)
            }
        }
    }

    /**
    Returns the specified event
    */
    #[tracing::instrument]
    pub async fn get_event_by_public_url(url: &str) -> Result<Event, APIError> {
        let full_page: String = reqwest::get(url)
            .inspect_err(|err| warn!("Failed to get full page: {:?}", err))
            .and_then(|res: Response| res.text())
            .await
            .expect("Error getting full page");
        let page_html = Html::parse_fragment(&full_page);
        let id_element = page_html
            .select(&EVENT_ID_SELECTOR)
            .next()
            .expect("Could not find ID element in page!")
            .attr("href")
            .and_then(|href| href.strip_prefix(AGENDA_PAGE_BY_ID_PATH))
            .expect("Could not find ID in element!")
            .parse()
            .expect("Fetched ID is not valid!");

        Self::get_event_by_id(id_element).await
    }

    #[tracing::instrument]
    async fn get_events_by_category(
        amount_per_page: Option<i32>,
        category: &str,
    ) -> serde_json::Result<Vec<EventResponse>> {
        let json_response = REST_CLIENT
            .get(format!(
                "{}?per_page={}&categories={}&type={}",
                AGENDA_EVENTS_URL,
                amount_per_page.unwrap_or(50000),
                category.to_lowercase(),
                EVENT_TYPE
            ))
            .send()
            .await
            .expect("Error sending request")
            .error_for_status()
            .expect("Request failed")
            .text()
            .await
            .expect("Received invalid response");

        serde_json::from_str::<Vec<EventResponse>>(&json_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agenda_cultural::dto::ResponseTitle;
    use serde_either::SingleOrVec;

    #[test_log::test(tokio::test)]
    async fn should_parse_event_by_date() {
        let february = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let march = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let events_per_month = AgendaCulturalAPI::parse_events_by_date(Vec::from([
            EventResponse {
                title: ResponseTitle {
                    rendered: "Como sobreviver a um acontecimento".to_string(),
                },
                subtitle: SingleOrVec::Single("".to_string()),
                description: vec![],
                featured_media_large: "".to_string(),
                link: "".to_string(),
                string_dates: "".to_string(),
                string_times: "".to_string(),
                start_date: march,
                venue: Default::default(),
                tags: Default::default(),
            },
            EventResponse {
                title: ResponseTitle {
                    rendered: "Sonho de uma noite de verão".to_string(),
                },
                subtitle: SingleOrVec::Single("".to_string()),
                description: vec![],
                featured_media_large: "".to_string(),
                link: "".to_string(),
                string_dates: "".to_string(),
                string_times: "".to_string(),
                start_date: february,
                venue: Default::default(),
                tags: Default::default(),
            },
            EventResponse {
                title: ResponseTitle {
                    rendered: "Mães".to_string(),
                },
                subtitle: SingleOrVec::Single("".to_string()),
                description: vec![],
                featured_media_large: "".to_string(),
                link: "".to_string(),
                string_dates: "".to_string(),
                string_times: "".to_string(),
                start_date: march,
                venue: Default::default(),
                tags: Default::default(),
            },
        ]))
        .await;

        assert_eq!(events_per_month.len(), 2);

        let february_events = events_per_month.get(&february).unwrap();
        let march_events = events_per_month.get(&march).unwrap();

        assert_eq!(february_events.len(), 1);
        assert_eq!(march_events.len(), 2);

        assert_eq!(february_events[0].title, "Sonho de uma noite de verão");
        assert_eq!(march_events[0].title, "Como sobreviver a um acontecimento");
        assert_eq!(march_events[1].title, "Mães");
    }
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
