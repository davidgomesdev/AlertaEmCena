const CHILDREN_TAG: &str = "crianças";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Event {
    pub title: String,
    pub details: EventDetails,
    pub link: String,
    pub occurring_at: Schedule,
    pub venue: String,
    pub tags: Vec<String>,
    pub is_for_children: bool,
}

impl Event {
    pub fn new(
        title: String,
        details: EventDetails,
        link: String,
        occurring_at: Schedule,
        venue: String,
        tags: Vec<String>,
    ) -> Self {
        Self {
            title,
            details,
            link,
            occurring_at,
            venue,
            is_for_children: tags.iter().any(|tag| tag.to_lowercase() == CHILDREN_TAG),
            tags,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
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
            image_url,
        }
    }
}

/// Portuguese Schedule information
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Schedule {
    pub dates: String,
    pub times: String,
}

impl Schedule {
    pub fn new(dates: String, times: String) -> Self {
        Self { dates, times }
    }
}

#[derive(strum::IntoStaticStr, Debug)]
pub enum Category {
    Teatro,
    Artes,
}
