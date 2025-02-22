use super::{dto::ResponseEvent, model::Event};
use crate::agenda_cultural::model::Category;
use lazy_static::lazy_static;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use std::collections::LinkedList;
use tracing::{error, info};

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
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
