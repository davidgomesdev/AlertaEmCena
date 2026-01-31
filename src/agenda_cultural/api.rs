use super::{dto::EventResponse, model::Event};
use crate::agenda_cultural::dto::SingleEventResponse;
use crate::agenda_cultural::model::Category;
use chrono::{Datelike, NaiveDate, TimeDelta, Utc};
use futures::TryFutureExt;
use lazy_static::lazy_static;
use reqwest::{Client, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::Jitter::Bounded;
use reqwest_retry::RetryTransientMiddleware;
use scraper::{Html, Selector};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Add;
use std::time::Duration;
use tracing::{debug, info, instrument, trace, warn};
use voca_rs::strip::strip_tags;

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";
const AGENDA_PAGE_BY_ID_PATH: &str = "https://www.agendalx.pt/?p=";
const EVENT_TYPE: &str = "event";
const DATE_PRINT_FORMAT: &str = "%Y-%m-%d";

lazy_static! {
    static ref REST_CLIENT: ClientWithMiddleware = ClientBuilder::new(Client::new())
        .with(RetryTransientMiddleware::new_with_policy(
            ExponentialBackoff::builder()
                .jitter(Bounded)
                .retry_bounds(Duration::from_millis(50), Duration::from_millis(1000))
                .build_with_total_retry_duration_and_max_retries(Duration::from_secs(10))
        ))
        .build();
    static ref EVENT_ID_SELECTOR: Selector = Selector::parse(&format!(
        r#"link[rel="shortlink"][href^="{}"]"#,
        AGENDA_PAGE_BY_ID_PATH
    ))
    .unwrap();
    static ref EVENT_DESCRIPTION_SELECTOR: Selector =
        Selector::parse(".entry-container > :not(.event__extra-info):not(.section-title):not(.section-title--venue):not(.venue):not(.post__share)").unwrap();
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
        let parsed_response = Self::get_events_by_category(amount_per_page, category).await?;

        info!("Fetched {} events", parsed_response.len());

        let events = Self::parse_events_by_date(parsed_response).await;

        Ok(events)
    }

    #[instrument(skip_all)]
    async fn parse_events_by_date(response: Vec<EventResponse>) -> BTreeMap<NaiveDate, Vec<Event>> {
        let mut events_by_date: BTreeMap<NaiveDate, Vec<Event>> = BTreeMap::new();

        Self::fill_incoming_months(&response, &mut events_by_date);

        debug!(
            "Found these months of events: {:?}",
            events_by_date.keys().cloned().collect::<Vec<NaiveDate>>()
        );

        for response in response
            .iter()
            .filter(|event| event.start_date != NaiveDate::MIN)
        {
            let model = Self::convert_response_to_model(response).await;
            let date = response.start_date.with_day(1).unwrap();

            if let Some(events) = events_by_date.get_mut(&date) {
                events.push(model);
            } else {
                warn!(
                    event = model.link,
                    date = date.format(DATE_PRINT_FORMAT).to_string(),
                    "The date of event '{}' was not in the list! (when it should)",
                    model.title
                );
                events_by_date.insert(date, Vec::from([model]));
            }
        }

        info!("Parsed events");

        events_by_date
    }

    #[instrument(skip_all)]
    fn fill_incoming_months(
        response: &[EventResponse],
        events_by_date: &mut BTreeMap<NaiveDate, Vec<Event>>,
    ) {
        let max_month = response
            .iter()
            .max_by(|first, second| first.start_date.cmp(&second.start_date));

        if let Some(last_event) = max_month {
            let max_date = NaiveDate::from_ymd_opt(
                last_event.start_date.year(),
                last_event.start_date.month(),
                1,
            )
            .unwrap();
            let mut min_date = Utc::now().date_naive().with_day(1).unwrap();

            trace!("Filling up until {:?}", max_date);
            while min_date.cmp(&max_date) != Ordering::Greater {
                events_by_date.insert(min_date, Vec::from([]));
                trace!("Going for {:?}", min_date);
                min_date = min_date.add(TimeDelta::days(31)).with_day(1).unwrap();
            }
        }
    }

    #[instrument(skip(response), fields(event = %response.link))]
    async fn convert_response_to_model(response: &EventResponse) -> Event {
        let description = Self::get_full_description(&response.link)
            .await
            .unwrap_or_else(|| {
                let preview_description = Self::clean_description(&response.description.concat());

                warn!(
                    "Unable to get full description. Using only preview description ({})",
                    preview_description
                );

                preview_description
            });

        response.to_model(description).await
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
            .map_err(APIError::ErrorSending)?
            .error_for_status()
            .map_err(APIError::ResponseError)?
            .text()
            .map_err(APIError::InvalidResponse)
            .await?;
        trace!("Json response: {json_response}");

        let parsed_response = serde_json::from_str::<SingleEventResponse>(&json_response)
            .map_err(APIError::ParseError)?;

        Ok(Self::convert_response_to_model(&parsed_response.event).await)
    }

    /**
    Returns the specified event
    */
    #[tracing::instrument]
    pub async fn get_event_by_public_url(url: &str) -> Result<Event, APIError> {
        let full_page: String = REST_CLIENT
            .get(url)
            .send()
            .map_err(APIError::ErrorSending)
            .await?
            .error_for_status()
            .map_err(APIError::ResponseError)?
            .text()
            .map_err(APIError::InvalidResponse)
            .await?;
        let page_html = Html::parse_fragment(&full_page);
        let id_element = page_html
            .select(&EVENT_ID_SELECTOR)
            .next()
            .ok_or_else(|| {
                APIError::FailedParsingHtml("Could not find ID element in page!".to_string())
            })?
            .attr("href")
            .and_then(|href| href.strip_prefix(AGENDA_PAGE_BY_ID_PATH))
            .ok_or_else(|| {
                APIError::FailedParsingHtml("Could not find ID in the element!".to_string())
            })?
            .parse()
            .map_err(|_| APIError::FailedParsingHtml("Fetched ID is not valid!".to_string()))?;

        Self::get_event_by_id(id_element).await
    }

    #[tracing::instrument]
    async fn get_events_by_category(
        amount_per_page: Option<i32>,
        category: &str,
    ) -> Result<Vec<EventResponse>, APIError> {
        let json_response = REST_CLIENT
            .get(format!(
                "{}?per_page={}&categories={}&type={}",
                AGENDA_EVENTS_URL,
                amount_per_page.unwrap_or(50000),
                category.to_lowercase(),
                EVENT_TYPE
            ))
            .send()
            .map_err(APIError::ErrorSending)
            .await?
            .error_for_status()
            .map_err(APIError::ResponseError)?
            .text()
            .map_err(APIError::InvalidResponse)
            .await?;

        serde_json::from_str::<Vec<EventResponse>>(&json_response).map_err(APIError::ParseError)
    }

    async fn get_full_description(link: &str) -> Option<String> {
        let full_page: Result<Response, _> = REST_CLIENT.get(link).send().await;

        match full_page {
            Ok(full_page) => {
                let description = full_page
                    .text()
                    .await
                    .inspect_err(|err| warn!("Failed to get full page text: {}", err));

                if description.is_err() {
                    return None;
                }

                Self::extract_full_description(&description.unwrap())
            }
            Err(err) => {
                warn!("Failed to get full page: {:?}", err);
                None
            }
        }
    }

    fn extract_full_description(full_page: &str) -> Option<String> {
        let page_html = Html::parse_fragment(full_page);

        let description_elements = page_html
            .select(&EVENT_DESCRIPTION_SELECTOR)
            .map(|p| p.inner_html().to_string())
            .collect::<Vec<String>>();

        if description_elements.is_empty() {
            return None;
        }

        let full_description = Self::clean_description(&description_elements.join("\n\n"));

        Some(full_description)
    }

    fn clean_description(description: &str) -> String {
        strip_tags(description)
            .replace("&nbsp;", " ")
            .trim_end_matches("\n")
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agenda_cultural::dto::ResponseTitle;
    use serde_either::SingleOrVec;
    use std::fs::read_to_string;

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

    #[test_log::test]
    fn should_extract_full_description() {
        let event_page =
            read_to_string("res/tests/event_page.html").expect("Could not get test resource");
        let actual = AgendaCulturalAPI::extract_full_description(&event_page);

        assert!(actual.is_some());
        assert_eq!(
            actual.unwrap(),
            read_to_string("res/tests/event_page_full_description.txt")
                .expect("Could not get test resource")
        );
    }

    #[test_log::test]
    fn should_extract_full_description_with_italic_description() {
        let event_page = read_to_string("res/tests/event_page_with_italic_description.html")
            .expect("Could not get test resource");
        let actual = AgendaCulturalAPI::extract_full_description(&event_page);

        assert!(actual.is_some());
        assert_eq!(
            actual.unwrap(),
            read_to_string("res/tests/event_page_full_description_with_italic_description.txt")
                .expect("Could not get test resource")
        );
    }
}

#[derive(Debug)]
pub enum APIError {
    ErrorSending(reqwest_middleware::Error),
    ResponseError(reqwest::Error),
    InvalidResponse(reqwest::Error),
    ParseError(serde_json::Error),
    FailedParsingHtml(String),
}
