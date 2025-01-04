use tracing::error;

use super::{dto::ResponseEvent, model::Event};

const AGENDA_EVENTS_URL: &str = "https://www.agendalx.pt/wp-json/agendalx/v1/events";

pub async fn get_events() -> Result<Vec<Event>, APIError> {
    let json_response = reqwest::get(format!("{}?search=&per_page=10&categories=teatro&type=event", AGENDA_EVENTS_URL))
        .await.unwrap().text().await.unwrap();
    let parsed_response = serde_json::from_str::<Vec<ResponseEvent>>(&json_response);

    return parsed_response
        .map(|mut res| res.iter_mut().map(|e| e.to_model()).collect())
        .map_err(|e| {
            error!("API response cannot be parsed! {}", e);
            APIError::InvalidResponse
        });
}

#[derive(Debug)]
pub enum APIError {
    InvalidResponse,
}
