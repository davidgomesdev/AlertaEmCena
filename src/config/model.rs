use serenity::all::ChannelId;

pub struct Config {
    pub debug_config: DebugConfig,
    pub teatro_channel_id: ChannelId,
    pub artes_channel_id: ChannelId,
}

pub struct DebugConfig {
    pub clear_channel: bool,
}
