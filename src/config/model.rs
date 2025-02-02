use serenity::all::ChannelId;

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
    pub event_limit: Option<i32>,
}

#[derive(Debug)]
pub struct EmojiConfig {
    pub id: i64,
    pub name: String,
}
