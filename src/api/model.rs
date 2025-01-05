#[allow(dead_code)]
#[derive(Debug)]
pub struct Event {
    pub event_type: String,
    pub title: String,
    pub details: EventDetails,
    pub link: String,
    pub occurring_at: Schedule,
    pub venue: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EventDetails {
    pub subtitle: String,
    pub description: String,
    pub image_url: String,
}

/// Portuguese Schedule information
#[allow(dead_code)]
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
