use serenity::all::ChannelId;

pub struct Config {
    pub debug_config: DebugConfig,
    pub teatro_channel_id: ChannelId,
    pub artes_channel_id: ChannelId,
    pub voting_emojis: [EmojiConfig; 5],
}

pub struct DebugConfig {
    pub clear_channel: bool,
    pub event_limit: Option<i32>,
}

pub struct EmojiConfig {
    pub id: i64,
    pub name: String,
}
