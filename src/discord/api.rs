use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use chrono::{Datelike, NaiveDate};
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::ReactionType::{Custom, Unicode};
use serenity::all::{
    AutoArchiveDuration, ChannelType, Colour, CreateEmbedAuthor, CreateThread, CurrentUser,
    EditThread, Embed, GatewayIntents, GetMessages, GuildChannel, Message, MessageId,
    MessageReaction, PartialGuild, PrivateChannel, ReactionType, User,
};
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::cache::Settings;
use serenity::model::error::Error;
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;
use std::fmt::Debug;
use tracing::field::debug;
use tracing::{debug, error, info, instrument, trace, warn};

const AUTHOR_NAME: &str = "AlertaEmCena";

const PORTUGUESE_MONTHS: [&str; 12] = [
    "Janeiro",
    "Fevereiro",
    "Março",
    "Abril",
    "Maio",
    "Junho",
    "Julho",
    "Agosto",
    "Setembro",
    "Outubro",
    "Novembro",
    "Dezembro",
];

const CHILDREN_LABEL: &str = "🧸 para crianças";

lazy_static! {
    static ref USER_MENTION_REGEX: Regex =
        Regex::new("<@(\\d+)>").expect("Failed to create mention regex");
}

pub struct DiscordAPI {
    pub client: Client,
    pub own_user: CurrentUser,
}

impl DiscordAPI {
    pub async fn default() -> Self {
        DiscordAPI::new(
            &env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set"),
            true,
        )
        .await
    }

    pub async fn new(token: &str, cache_flag: bool) -> Self {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGE_REACTIONS;
        let mut cache_settings = Settings::default();

        cache_settings.cache_channels = cache_flag;

        let client = Client::builder(token, intents)
            .cache_settings(cache_settings)
            .await
            .expect("Error creating discord client");
        let own_user = client
            .http
            .get_current_user()
            .await
            .expect("Error getting user");

        debug!("Own user id is {}", own_user.id);

        Self { client, own_user }
    }

    pub async fn get_messages(&self, channel_id: ChannelId) -> Vec<Message> {
        channel_id
            .messages_iter(&self.client.http)
            .filter_map(|a| async { a.ok() })
            .collect::<Vec<Message>>()
            .await
    }

    #[instrument(skip(self, channel_id), fields(channel_id = %channel_id.to_string(), event = %event.title.to_string()
    ))]
    pub async fn send_event(&self, channel_id: ChannelId, event: Event) -> Message {
        info!("Sending event");

        let mut description = event.details.description;

        if event.is_for_children {
            description = format!("{}\n\n{}", description.clone(), CHILDREN_LABEL);
        }

        let embed = CreateEmbed::new()
            .title(event.title)
            .url(event.link)
            .description(description.clone())
            .author(CreateEmbedAuthor::new(AUTHOR_NAME))
            .color(Colour::new(0x005eeb))
            .field("Datas", event.occurring_at.dates, true)
            .field("Onde", event.venue, true)
            .image(event.details.image_url);

        let message_builder = CreateMessage::new().add_embed(embed.clone());

        match channel_id
            .send_message(&self.client.http, message_builder)
            .await
        {
            Ok(message) => message,
            Err(err) => {
                if let serenity::Error::Model(Error::EmbedTooLarge(_)) = err {
                    info!("Couldn't send embed with full description, retrying with a shorter description");

                    let first_line = description.lines().next();
                    let short_description: String = first_line
                        .unwrap_or(&description)
                        .chars()
                        .take(4000)
                        .collect();
                    let message_builder =
                        CreateMessage::new().add_embed(embed.description(short_description));

                    channel_id
                        .send_message(&self.client.http, message_builder)
                        .await
                        .expect("Failed to send message")
                } else {
                    panic!("Failed sending message due to '{}'", err);
                }
            }
        }
    }

    #[instrument(skip(self, message, emoji))]
    pub async fn add_custom_reaction(&self, message: &Message, emoji: &EmojiConfig) {
        trace!("Adding reaction");

        match message
            .react(
                &self.client.http,
                Custom {
                    animated: false,
                    id: emoji
                        .id
                        .to_string()
                        .parse()
                        .expect("Invalid emoji ID format"),
                    name: Some(emoji.name.to_string()),
                },
            )
            .await
        {
            Ok(_) => {
                trace!("Successfully added '{}' reaction", emoji.name);
            }
            Err(err) => {
                error!(
                    "Failed to add '{}' ID {} reaction on message: id={} {:?}",
                    emoji.name, emoji.id, message.id, err
                );
            }
        }
    }

    pub async fn get_all_messages(&self, channel_id: ChannelId) -> serenity::Result<Vec<Message>> {
        channel_id
            .messages_iter(&self.client.http)
            .try_collect()
            .await
    }

    #[instrument(skip(self, message), fields(event = %message.embeds.first().map(|embed| embed.url.clone().unwrap()).unwrap_or_default()
    ))]
    pub async fn add_reaction_to_message(&self, message: &Message, emoji_char: char) {
        let react_result = message
            .react(&self.client.http, ReactionType::from(emoji_char))
            .await;

        if let Err(e) = react_result {
            let msg = &format!("Failed to add reaction {} to message", emoji_char);
            error!(
                msg,
                error = %e
            );
        }
    }

    #[instrument(skip(self, message), fields(event = %message.embeds.first().map(|embed| embed.url.clone().unwrap()).unwrap_or_default()
    ))]
    pub async fn tag_save_for_later_reactions(&self, message: &mut Message, emoji_char: char) {
        let save_for_later_reaction = ReactionType::from(emoji_char);

        // Is empty ensures no one has ever saved for later,
        //      message is fresh (no need to remove mentions)
        // Helps avoid calling the API for reaction_users, improving performance
        if message.content.is_empty()
            && Self::has_no_user_emoji_reaction(message, &emoji_char.to_string())
        {
            trace!("No user has ever saved for later");
            return;
        }

        let saved_for_later_user_ids: Vec<String> = message
            .reaction_users(&self.client.http, save_for_later_reaction, None, None)
            .await
            .map(|users| {
                users
                    .into_iter()
                    .map(|user| user.id.to_string())
                    .filter(|user_id| *user_id != self.own_user.id.to_string())
                    .collect()
            })
            .expect("Couldn't get users that reacted!");

        if saved_for_later_user_ids.is_empty() && message.content.is_empty() {
            trace!("No users saved for later");
            return;
        }

        let mentions = saved_for_later_user_ids
            .iter()
            .map(|user_id| format!("<@{}>", user_id))
            .collect::<Vec<String>>()
            .join(" ");
        let message_content = format!("Interessados: {}", mentions);

        if saved_for_later_user_ids.is_empty() && message.pinned {
            message
                .unpin(&self.client.http)
                .await
                .expect("Failed to unpin no longer saved for later message!");
        }

        if !saved_for_later_user_ids.is_empty() && !message.pinned {
            message
                .pin(&self.client.http)
                .await
                .expect("Failed to pin saved for later message!");
        }

        if message_content.trim() == message.content.trim() {
            trace!("No new users saved for later");
            return;
        }

        info!("Saved for later changed to '{}'", mentions);

        let mut edit_message = EditMessage::new().content(message_content);

        if saved_for_later_user_ids.is_empty() {
            edit_message = edit_message.content("");
        }

        message
            .edit(&self.client.http, edit_message)
            .await
            .expect("Failed to edit message!");
    }

    #[instrument(skip(self, event_message, vote_emojis), fields(event = %event_message.embeds.first().map(|embed| embed.url.clone().unwrap()).unwrap_or_default()
    ))]
    pub async fn send_privately_users_review(
        &self,
        event_message: &Message,
        vote_emojis: &[EmojiConfig; 5],
    ) {
        let mut event_embed = event_message.embeds.first().cloned().unwrap();
        let event_url = event_embed.url.clone();

        if event_url.is_none() {
            warn!("Event has no URL!");
            return;
        }

        let users_votes = self.get_user_votes(event_message, vote_emojis).await;

        if users_votes.is_empty() {
            trace!("No user has voted on this message");
            return;
        }

        let event_url = event_url.unwrap();

        event_embed.fields = Vec::new();

        for (vote, users) in users_votes.iter().enumerate() {
            for user in users.iter().filter(|user| !user.bot) {
                self.send_user_review(user, &event_url, event_embed.clone(), vote_emojis, vote)
                    .await;
            }
        }
    }

    async fn send_user_review(
        &self,
        user: &User,
        event_url: &str,
        event_embed: Embed,
        vote_emojis: &[EmojiConfig; 5],
        vote: usize,
    ) {
        match user.create_dm_channel(&self.client.http).await {
            Ok(dm) => {
                trace!("Found user {} with vote {}", user.id, vote + 1);

                if !self.is_event_sent_in_dm(event_url, &dm).await {
                    info!("Sent vote {} for user {}", user.id, vote + 1);
                    self.send_user_review_in_dm(&vote_emojis[vote], event_embed, &dm)
                        .await;
                }
            }
            Err(error) => {
                warn!(
                    "Couldn't create DM channel for user '{}' due to: {}",
                    user.name, error
                );
            }
        }
    }

    async fn get_user_votes(
        &self,
        event_message: &Message,
        vote_emojis: &[EmojiConfig; 5],
    ) -> [Vec<User>; 5] {
        let mut users_votes: [Vec<User>; 5] = [vec![], vec![], vec![], vec![], vec![]];

        for (index, voting_emoji) in vote_emojis.iter().enumerate() {
            if Self::has_no_user_votes(event_message, voting_emoji) {
                continue;
            }

            let users_that_reacted: Vec<User> = event_message
                .reaction_users(
                    &self.client.http,
                    Custom {
                        animated: false,
                        id: voting_emoji
                            .id
                            .to_string()
                            .parse()
                            .expect("Invalid emoji ID format"),
                        name: Some(voting_emoji.name.to_string()),
                    },
                    None,
                    None,
                )
                .await
                .expect("Couldn't get users that reacted!");

            for user in users_that_reacted {
                if user.id == self.own_user.id {
                    continue;
                }

                users_votes[index].push(user);
            }
        }

        users_votes
    }

    fn has_no_user_votes(event_message: &Message, voting_emoji: &EmojiConfig) -> bool {
        let reaction = event_message.reactions.iter().find(|reaction| {
            if let Custom { id, .. } = reaction.reaction_type {
                id == voting_emoji.id
            } else {
                false
            }
        });

        if let Some(reaction) = reaction {
            Self::has_no_user_reactions(reaction)
        } else {
            warn!(
                "Message does not have reaction emoji '{}'!",
                voting_emoji.name
            );
            false
        }
    }

    fn has_no_user_emoji_reaction(event_message: &Message, emoji_char: &str) -> bool {
        let reaction = event_message.reactions.iter().find(|reaction| {
            if let Unicode(char) = &reaction.reaction_type {
                *char == emoji_char
            } else {
                false
            }
        });

        if let Some(reaction) = reaction {
            Self::has_no_user_reactions(reaction)
        } else {
            warn!("Message does not have saved for later emoji!");
            false
        }
    }

    fn has_no_user_reactions(reaction: &MessageReaction) -> bool {
        if reaction.count == 1 {
            // No one has voted
            if reaction.me {
                return true;
            } else {
                warn!("Self did not react!")
            }
        }
        false
    }

    #[instrument(skip(self, vote_emoji, event_embed, dm), fields(user_name = %dm.recipient.name.to_string(), vote = %vote_emoji.name.to_string(), event_url = event_embed.url))]
    async fn send_user_review_in_dm(
        &self,
        vote_emoji: &EmojiConfig,
        event_embed: Embed,
        dm: &PrivateChannel,
    ) {
        info!("Sending vote");

        let comment = self.get_user_last_comment(dm).await;
        let description = event_embed.description.clone();

        let embed = Self::create_user_review_embed(vote_emoji, event_embed, &comment, description);

        dm.send_message(&self.client.http, CreateMessage::new().embed(embed))
            .await
            .expect("Failed to send message");

        if let Some(comment) = comment {
            self.add_reaction_to_message(&comment, '✅').await;
        }
    }

    #[instrument(skip(vote_emoji, event_embed, comment, description), fields(event_name = %event_embed.title.clone().unwrap_or_default()))]
    fn create_user_review_embed(
        vote_emoji: &EmojiConfig,
        event_embed: Embed,
        comment: &Option<Message>,
        description: Option<String>,
    ) -> CreateEmbed {
        match &comment {
            None => CreateEmbed::from(event_embed).description(format!(
                "{}\n**Voto:** {}",
                description.unwrap(),
                vote_emoji
            )),
            Some(comment) => CreateEmbed::from(event_embed).description(format!(
                "{}\n**Voto:** {}\n**Comentários:** {}",
                description.unwrap(),
                vote_emoji,
                comment.content
            )),
        }
    }

    #[instrument(skip(self, dm), fields(user_name = %dm.recipient.name.to_string()))]
    async fn get_user_last_comment(&self, dm: &PrivateChannel) -> Option<Message> {
        match dm.last_message_id {
            Some(last_message_id) => {
                self.client
                    .http
                    .get_message(dm.id, last_message_id)
                    .await
                    .inspect_err(|e| {
                        warn!("Failed to get last message: {}", e);
                    })
                    .ok()
                    .take_if(|msg| msg.author != *self.own_user)
                    // a reply will be used in another feature
                    .take_if(|msg| {
                        let is_a_reply = msg.referenced_message.is_some();

                        if is_a_reply {
                            debug!("Ignoring last message since it's reply to another");
                        }

                        !is_a_reply
                    })
            }
            None => None,
        }
    }

    #[instrument(skip(self, dm), fields(dm_id = %dm.id.to_string()))]
    async fn is_event_sent_in_dm(&self, event_url: &str, dm: &PrivateChannel) -> bool {
        let mut last_message_id: Option<MessageId> = None;
        let mut searched_all_dms = false;
        let mut is_found = false;

        while !searched_all_dms {
            let mut filter = GetMessages::default();

            if let Some(last_message_id) = last_message_id {
                filter = filter.before(last_message_id)
            }

            let messages_iter = dm
                .messages(&self.client.http, filter)
                .await
                .expect("Couldn't get dm message!");

            if messages_iter.iter().any(|msg| {
                msg.embeds
                    .first()
                    .and_then(|embed| embed.url.clone())
                    .unwrap_or_default()
                    == event_url
            }) {
                is_found = true;
                break;
            }

            match messages_iter.first() {
                None => {
                    searched_all_dms = true;
                }
                Some(last_message) => last_message_id = Some(last_message.id),
            }
        }

        is_found
    }

    #[instrument(skip(self, channel_id), fields(channel_id = %channel_id.to_string()))]
    pub async fn get_guild(&self, channel_id: ChannelId) -> PartialGuild {
        let guild_channel = channel_id
            .to_channel(&self.client.http)
            .await
            .expect("Could not get channel")
            .guild()
            .expect("Channel does not appear to of a guild");
        guild_channel
            .guild_id
            .to_partial_guild(&self.client.http)
            .await
            .unwrap()
    }

    #[instrument(skip_all, fields(guild_id = %guild.id.to_string(), channel_id = %channel_id.to_string()))]
    pub async fn get_channel_threads(
        &self,
        guild: &PartialGuild,
        channel_id: ChannelId,
    ) -> Vec<GuildChannel> {
        self.unarchive_archived_threads(channel_id).await;

        debug("Unarchived archived threads");

        let active_threads: Vec<GuildChannel> = guild
            .get_active_threads(&self.client.http)
            .await
            .unwrap()
            .threads
            .into_iter()
            .filter(|thread| thread.parent_id == Some(channel_id))
            .collect();

        debug!(
            "Found threads: [{:?}]",
            Self::concat_thread_names(&active_threads)
        );

        active_threads
    }

    async fn unarchive_archived_threads(&self, channel_id: ChannelId) {
        let mut archived_threads = channel_id
            .get_archived_public_threads(&self.client.http, None, None)
            .await
            .expect("Could not get archived threads")
            .threads;

        debug!(
            "Found archived threads: [{:?}]",
            Self::concat_thread_names(&archived_threads)
        );

        for thread in &mut archived_threads {
            thread
                .edit_thread(&self.client.http, EditThread::new().archived(false))
                .await
                .expect("Failed to unarchive archived threads!")
        }
    }

    fn concat_thread_names(threads: &[GuildChannel]) -> String {
        threads.iter().map(|thread| thread.name.as_str()).join(",")
    }

    #[instrument(skip(self, threads, channel_id), fields(thread_count = %threads.len(), channel_id = %channel_id.to_string()))]
    pub async fn get_date_thread(
        &self,
        threads: &[GuildChannel],
        channel_id: ChannelId,
        date: NaiveDate,
    ) -> EventsThread {
        let year = date.year();
        let month_in_portuguese = month_to_portuguese_display(&date);

        for thread in threads {
            if thread.name == format!("{month_in_portuguese} {year}") {
                return EventsThread::new(thread.id);
            }
        }

        EventsThread::new(
            channel_id
                .create_thread(
                    &self.client.http,
                    CreateThread::new(format!("{month_in_portuguese} {year}"))
                        .kind(ChannelType::PublicThread)
                        .auto_archive_duration(AutoArchiveDuration::OneWeek),
                )
                .await
                .unwrap()
                .id,
        )
    }

    #[instrument(skip(self, channel_id), fields(channel_id = %channel_id.to_string()))]
    pub async fn get_event_urls_sent(&self, channel_id: ChannelId) -> Vec<String> {
        channel_id
            .messages_iter(&self.client.http)
            .map::<_, fn(_) -> Vec<Embed>>(|message: Result<Message, SerenityError>| {
                message.expect("Error getting message").embeds
            })
            .concat()
            .await
            .iter()
            .filter_map(|embed| embed.url.clone())
            .collect()
    }

    #[instrument(skip(self, channel_id), fields(channel_id = %channel_id.to_string()))]
    pub async fn delete_all_messages(&self, channel_id: &ChannelId) {
        let messages = channel_id
            .messages_iter(&self.client.http)
            .try_collect::<Vec<Message>>()
            .await
            .expect("Failed to fetch messages");

        self.delete_messages(channel_id, &messages).await;

        let guild = self.get_guild(*channel_id).await;
        let threads = self.get_channel_threads(&guild, *channel_id).await;

        for thread in threads {
            thread
                .delete(&self.client.http)
                .await
                .expect("Failed to delete threads!");
        }
    }

    #[instrument(skip(self, channel_id, messages), fields(message_count = %messages.len(), channel_id = %channel_id.to_string()))]
    async fn delete_messages(&self, channel_id: &ChannelId, messages: &[Message]) {
        for chunk in messages.chunks(100) {
            debug!("Deleting {} messages", chunk.len());
            let deletion_result = channel_id.delete_messages(&self.client.http, chunk).await;

            if let Err(err) = deletion_result {
                warn!("Failed due to: '{}'. Retrying individually", err);

                for msg in chunk {
                    msg.delete(&self.client.http)
                        .await
                        .expect("Failed to delete one of the messages individually");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn when_no_user_has_voted_other_than_bot_should_return_true() {
        let reaction = serde_json::from_str(
            r#"
        {
          "count": 1,
          "count_details": {
            "burst": 0,
            "normal": 1
          },
          "me": true,
          "me_burst": false,
          "emoji": { "id": null, "name": "1" },
          "burst_colors": []
        }
        "#,
        )
        .unwrap();
        let has_no_user_reactions = DiscordAPI::has_no_user_reactions(&reaction);

        assert!(has_no_user_reactions);
    }

    #[test_log::test]
    fn when_at_least_one_user_has_voted_other_than_bot_should_return_false() {
        let reaction = serde_json::from_str(
            r#"
        {
          "count": 2,
          "count_details": {
            "burst": 0,
            "normal": 2
          },
          "me": true,
          "me_burst": false,
          "emoji": { "id": null, "name": "1" },
          "burst_colors": []
        }
        "#,
        )
        .unwrap();
        let has_no_user_reactions = DiscordAPI::has_no_user_reactions(&reaction);

        assert!(!has_no_user_reactions);
    }

    #[test_log::test]
    fn when_one_user_has_voted_and_the_bot_has_not_should_return_false() {
        let reaction = serde_json::from_str(
            r#"
        {
          "count": 1,
          "count_details": {
            "burst": 0,
            "normal": 1
          },
          "me": false,
          "me_burst": false,
          "emoji": { "id": null, "name": "1" },
          "burst_colors": []
        }
        "#,
        )
        .unwrap();
        let has_no_user_reactions = DiscordAPI::has_no_user_reactions(&reaction);

        assert!(!has_no_user_reactions);
    }
}

pub fn month_to_portuguese_display(date: &NaiveDate) -> String {
    PORTUGUESE_MONTHS[(date.month() - 1) as usize].to_string()
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct EventsThread {
    pub thread_id: ChannelId,
}

impl EventsThread {
    pub fn new(channel_id: ChannelId) -> EventsThread {
        EventsThread {
            thread_id: channel_id,
        }
    }
}
