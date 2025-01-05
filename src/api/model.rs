use chrono::NaiveDate;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Event {
    pub event_type: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub link: String,
    pub occurring_at: DateRange,
    pub venue: String,
}

#[derive(Debug)]
pub struct DateRange {
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

#[derive(strum::IntoStaticStr)]
pub enum Category {
    Teatro,
    Artes
}
