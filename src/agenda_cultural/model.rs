#[allow(dead_code)]
#[derive(Debug)]
pub struct Event {
    pub title: String,
    pub details: EventDetails,
    pub link: String,
    pub occurring_at: Schedule,
    pub venue: String,
    pub tags: Vec<String>,
}

impl Event {
    pub fn new(title: String, details: EventDetails, link: String, occurring_at: Schedule, venue: String, tags: Vec<String>) -> Self {
        Self {
            title,
            details,
            link,
            occurring_at,
            venue,
            tags
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EventDetails {
    pub subtitle: String,
    pub description: String,
    pub image_url: String,
}

impl EventDetails {
    pub fn new(subtitle: String, description: String, image_url: String) -> Self {
        Self {
            subtitle,
            description,
            image_url
        }
    }
}

/// Portuguese Schedule information
#[allow(dead_code)]
#[derive(Debug)]
pub struct Schedule {
    pub dates: String,
    pub times: String
}

impl Schedule {
    pub fn new(dates: String, times: String) -> Self {
        Self {
            dates,
            times
        }
    }
}

#[derive(strum::IntoStaticStr, Debug)]
pub enum Category {
    Teatro,
    Artes
}
