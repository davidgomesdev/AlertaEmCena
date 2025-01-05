use super::{dto::ResponseEvent, model::Event};
use crate::api::model::Category;
use futures::future;

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";
const EVENT_TYPE: &str = "event";

/**
   Returns events with ascending order
*/
pub async fn get_events(amount_per_page: i32, category: &Category) -> Result<Vec<Event>, APIError> {
    let category: &'static str = category.into();
    let json_response = reqwest::get(format!(
        "{}?per_page={}&categories={}&type={}",
        AGENDA_EVENTS_URL,
        amount_per_page,
        category.to_lowercase(),
        EVENT_TYPE
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let parsed_response = serde_json::from_str::<Vec<ResponseEvent>>(&json_response);

    match parsed_response {
        Ok(mut parsed_response) => {
            Ok(future::join_all(parsed_response.iter_mut().rev().map(|e| e.to_model())).await)
        }
        Err(_) => Err(APIError::InvalidResponse),
    }
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
