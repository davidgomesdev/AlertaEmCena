use super::{dto::ResponseEvent, model::Event};
use crate::api::model::Category;
use tracing::error;

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";

/*
   Returns events with ascending order
*/
pub async fn get_events(amount_per_page: i32, category: &Category) -> Result<Vec<Event>, APIError> {
    let category: &'static str = category.into();
    let json_response = reqwest::get(format!(
        "{}?per_page={}&categories={}&type=event",
        AGENDA_EVENTS_URL,
        amount_per_page,
        category.to_lowercase()
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let parsed_response = serde_json::from_str::<Vec<ResponseEvent>>(&json_response);

    parsed_response
        .map(|mut res| res.iter_mut().map(|e| e.to_model()).rev().collect())
        .map_err(|e| {
            error!("API response cannot be parsed! {}", e);
            APIError::InvalidResponse
        })
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
