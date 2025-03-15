use super::{dto::ResponseEvent, model::Event};
use crate::agenda_cultural::dto::SingleEventResponse;
use crate::agenda_cultural::model::Category;
use futures::TryFutureExt;
use lazy_static::lazy_static;
use reqwest::{Client, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use scraper::{Html, Selector};
use std::collections::LinkedList;
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
    * amount_per_page: -1 will retrieve everything
    */
    #[tracing::instrument]
    pub async fn get_events(
        category: &Category,
        amount_per_page: Option<i32>,
    ) -> Result<LinkedList<Event>, APIError> {
        match amount_per_page {
            None => {
                info!("Getting all events");
            }
            Some(amount) => {
                info!("Getting {} events", amount);
            }
        }

        let category: &'static str = category.into();

        let json_response = REST_CLIENT
            .get(format!(
                "{}?per_page={}&categories={}&type={}",
                AGENDA_EVENTS_URL,
                amount_per_page.unwrap_or(-1),
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
        let parsed_response = serde_json::from_str::<Vec<ResponseEvent>>(&json_response);

        match parsed_response {
            Ok(parsed_response) => {
                let mut models = LinkedList::<Event>::new();

                for response in parsed_response.iter() {
                    models.push_back(response.to_model().await);
                }

                Ok(models)
            }
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
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
