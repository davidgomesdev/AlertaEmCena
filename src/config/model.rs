use serenity::all::ChannelId;
use std::fmt::Display;

#[derive(Debug)]
pub struct Config {
    pub debug_config: DebugConfig,
    pub teatro_channel_id: ChannelId,
    pub artes_channel_id: ChannelId,
    pub voting_emojis: [EmojiConfig; 5],
}

#[derive(Debug)]
pub struct DebugConfig {
    pub clear_channel: bool,
    pub exit_after_clearing: bool,
    pub skip_sending: bool,
    pub skip_feature_reactions: bool,
    pub skip_artes: bool,
    pub event_limit: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct EmojiConfig {
    pub id: i64,
    pub name: String,
}

impl Display for EmojiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<:{}:{}>", self.name, self.id)
    }
}
