use chrono::NaiveDate;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Event {
    pub event_type: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub link: String,
    pub occurring_at: Schedule,
    pub venue: String,
}

/// Portuguese Schedule information
#[derive(Debug)]
pub struct Schedule {
    pub dates: String,
    pub times: String
}

#[derive(strum::IntoStaticStr)]
pub enum Category {
    Teatro,
    Artes
}
