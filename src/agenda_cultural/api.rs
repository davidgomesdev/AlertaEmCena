use super::{dto::ResponseEvent, model::Event};
use crate::agenda_cultural::model::Category;
use futures::future;
use lazy_static::lazy_static;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use tracing::debug;

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";
const EVENT_TYPE: &str = "event";
const MAX_RETRIES: u32 = 5;

lazy_static! {
    static ref REST_CLIENT: ClientWithMiddleware = ClientBuilder::new(Client::new())
        .with(RetryTransientMiddleware::new_with_policy(
            ExponentialBackoff::builder().build_with_max_retries(MAX_RETRIES)
        ))
        .build();
}

pub struct AgendaCulturalAPI;

impl AgendaCulturalAPI {
    /**
    Returns events with ascending order
    */
    pub async fn get_events(
        amount_per_page: i32,
        category: &Category,
    ) -> Result<Vec<Event>, APIError> {
        debug!("Getting {} events", amount_per_page);

        let category: &'static str = category.into();
        let json_response = REST_CLIENT
            .get(format!(
                "{}?per_page={}&categories={}&type={}",
                AGENDA_EVENTS_URL,
                amount_per_page,
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
            Ok(mut parsed_response) => {
                Ok(future::join_all(parsed_response.iter_mut().rev().map(|e| {
                    e.to_model()
                })).await)
            }
            Err(_) => Err(APIError::InvalidResponse),
        }
    }
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
