use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use crate::metrics::{record_dm_review_sent, MetricResult};
use chrono::{Datelike, NaiveDate};
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::ReactionType::{Custom, Unicode};
use serenity::all::{
    AutoArchiveDuration, ChannelType, Colour, CreateEmbedAuthor, CreateThread, CurrentUser,
    EditThread, Embed, GatewayIntents, GetMessages, GuildChannel, Message, MessageId,
    MessageReaction, MessageType, PartialGuild, PrivateChannel, ReactionType, User, UserId,
};
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::cache::Settings;
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;
use std::fmt::Debug;
use tracing::field::debug;
use tracing::{debug, error, info, trace, warn};

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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DiscordError {
    Api,
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

    pub async fn send_event(
        &self,
        channel_id: ChannelId,
        event: Event,
        ticket_shop_url: Option<String>,
        ticket_shop_icon_url: &str,
    ) -> Result<Message, DiscordError> {
        info!(channel_id = %channel_id, event = %event.title, "Sending event");

        let mut description = event.details.description;

        if event.is_for_children {
            description = format!("{}\n\n{CHILDREN_LABEL}", description.clone());
        }

        let mut author = CreateEmbedAuthor::new(&event.venue);

        if let Some(ticket_shop_url) = ticket_shop_url {
            author = author.url(ticket_shop_url).icon_url(ticket_shop_icon_url);
        }

        let embed_description = Self::truncate_embed_description(description);
        let embed_title = event.title.clone();

        let embed = CreateEmbed::new()
            .title(embed_title)
            .url(event.link)
            .description(embed_description)
            .author(author)
            .color(Colour::new(0x005eeb))
            .field("Datas", event.occurring_at.dates, true)
            .image(event.details.image_url);

        let message_builder = CreateMessage::new().add_embed(embed.clone());

        channel_id
            .send_message(&self.client.http, message_builder)
            .await
            .map_err(|err| {
                error!("Failed sending event '{}' due to '{}'", event.title, err);
                DiscordError::Api
            })
    }

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

    pub async fn get_all_messages(&self, channel_id: ChannelId) -> Vec<Message> {
        channel_id
            .messages_iter(&self.client.http)
            .filter_map(|result| async {
                result
                    .inspect_err(|e| error!("Failed to fetch messages: {}", e))
                    .ok()
            })
            .collect()
            .await
    }

    pub async fn add_reaction_to_message(&self, message: &Message, emoji_char: char) {
        let react_result = message
            .react(&self.client.http, ReactionType::from(emoji_char))
            .await;

        debug!(
            emoji = %emoji_char,
            event_url = %message
                .embeds
                .first()
                .and_then(|embed| embed.url.as_deref())
                .unwrap_or("no_url"),
            message_id = %message.id,
            "Added reaction to message"
        );

        if let Err(e) = react_result {
            let msg = &format!("Failed to add reaction {} to message", emoji_char);
            error!(
                msg,
                error = %e
            );
        }
    }

    /// Returns whether this call resulted in the message being newly pinned.
    pub async fn tag_save_for_later_reactions(
        &self,
        message: &mut Message,
        emoji_char: char,
    ) -> bool {
        let save_for_later_reaction = ReactionType::from(emoji_char);

        // Is empty ensures no one has ever saved for later,
        //      message is fresh (no need to remove mentions)
        // Helps avoid calling the API for reaction_users, improving performance
        if message.content.is_empty()
            && Self::has_no_user_emoji_reaction(message, &emoji_char.to_string())
        {
            trace!("No user has ever saved for later");
            return false;
        }

        let saved_for_later_user_ids: Vec<String> = match message
            .reaction_users(&self.client.http, save_for_later_reaction, None, None)
            .await
        {
            Ok(users) => users
                .into_iter()
                .map(|user| user.id.to_string())
                .filter(|user_id| *user_id != self.own_user.id.to_string())
                .collect(),
            Err(e) => {
                error!("Failed to get save-for-later reaction users: {}", e);
                return false;
            }
        };

        if saved_for_later_user_ids.is_empty() && message.content.is_empty() {
            trace!("No users saved for later");
            return false;
        }

        let mentions = saved_for_later_user_ids
            .iter()
            .map(|user_id| format!("<@{}>", user_id))
            .collect::<Vec<String>>()
            .join(" ");
        let message_content = format!("Interessados: {}", mentions);

        let mut newly_pinned = false;

        if saved_for_later_user_ids.is_empty() && message.pinned {
            if let Err(e) = message.unpin(&self.client.http).await {
                error!("Failed to unpin message {}: {}", message.id, e);
            }
        }

        if !saved_for_later_user_ids.is_empty() && !message.pinned {
            match message.pin(&self.client.http).await {
                Ok(_) => newly_pinned = true,
                Err(e) => error!("Failed to pin message {}: {}", message.id, e),
            }
        }

        if message_content.trim() == message.content.trim() {
            trace!("No new users saved for later");
            return newly_pinned;
        }

        info!("Saved for later changed to '{}'", mentions);

        let mut edit_message = EditMessage::new().content(message_content);

        if saved_for_later_user_ids.is_empty() {
            edit_message = edit_message.content("");
        }

        if let Err(e) = message.edit(&self.client.http, edit_message).await {
            error!(
                "Failed to edit save-for-later message {}: {}",
                message.id, e
            );
        }

        newly_pinned
    }

    /// Deletes the "X pinned a message" system message(s) left behind after pinning,
    /// for a thread where `pin_count` pins were performed in this run.
    pub async fn delete_pin_notifications(&self, channel_id: ChannelId, pin_count: usize) {
        if pin_count == 0 {
            return;
        }

        let mut pin_notifications = self.find_pin_notifications(channel_id, pin_count).await;

        if pin_notifications.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            pin_notifications = self.find_pin_notifications(channel_id, pin_count).await;
        }

        if pin_notifications.is_empty() {
            warn!(
                "Could not find pin notification message(s) in channel {} after retry, ignoring",
                channel_id
            );
            return;
        }

        for message in pin_notifications {
            if let Err(e) = message.delete(&self.client.http).await {
                error!(
                    "Failed to delete pin notification message {}: {}",
                    message.id, e
                );
            }
        }
    }

    async fn find_pin_notifications(&self, channel_id: ChannelId, limit: usize) -> Vec<Message> {
        match channel_id
            .messages(&self.client.http, GetMessages::new().limit(limit as u8))
            .await
        {
            Ok(messages) => messages
                .into_iter()
                .filter(|m| m.kind == MessageType::PinsAdd)
                .collect(),
            Err(e) => {
                error!(
                    "Failed to fetch messages from channel {} to find pin notification: {}",
                    channel_id, e
                );
                Vec::new()
            }
        }
    }

    pub async fn send_privately_users_review(
        &self,
        event_message: &Message,
        vote_emojis: &[EmojiConfig; 5],
    ) -> Vec<UserId> {
        let mut users_with_reviews = Vec::new();
        let mut event_embed = event_message.embeds.first().cloned().unwrap();
        let event_url = event_embed.url.clone();

        if event_url.is_none() {
            warn!("Event has no URL!");
            return users_with_reviews;
        }

        let users_votes = self.get_user_votes(event_message, vote_emojis).await;

        if users_votes.is_empty() {
            trace!("No user has voted on this message");
            return users_with_reviews;
        }

        let event_url = event_url.unwrap();

        event_embed.fields = Vec::new();

        for (vote, users) in users_votes.iter().enumerate() {
            for user in users.iter().filter(|user| !user.bot) {
                if !users_with_reviews.contains(&user.id) {
                    users_with_reviews.push(user.id);
                }
                self.send_user_review(user, &event_url, event_embed.clone(), vote_emojis, vote)
                    .await;
            }
        }

        users_with_reviews
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

                match self.is_event_sent_in_dm(event_url, &dm).await {
                    Ok(false) => {
                        info!("Sent vote {} for user {}", user.id, vote + 1);
                        self.send_user_review_in_dm(&vote_emojis[vote], event_embed, &dm)
                            .await;
                    }
                    Ok(true) => {
                        trace!("Event already sent to user {}", user.id);
                    }
                    Err(_) => {
                        // error already logged inside is_event_sent_in_dm
                    }
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

            let users_that_reacted: Vec<User> = match event_message
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
            {
                Ok(users) => users,
                Err(e) => {
                    error!(
                        "Failed to get reaction users for emoji '{}': {}",
                        voting_emoji.name, e
                    );
                    continue;
                }
            };

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
            Self::has_someone_reacted(reaction)
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
            Self::has_someone_reacted(reaction)
        } else {
            warn!("Message does not have saved for later emoji!");
            false
        }
    }

    fn has_someone_reacted(reaction: &MessageReaction) -> bool {
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

    async fn send_user_review_in_dm(
        &self,
        vote_emoji: &EmojiConfig,
        event_embed: Embed,
        dm: &PrivateChannel,
    ) {
        info!(
            user_name = %dm.recipient.name,
            vote_emoji = %vote_emoji,
            event = %event_embed.title.as_deref().unwrap_or("no_title"),
            "Sending vote"
        );

        let comment = self.get_user_last_comment(dm).await;

        let embed = Self::create_user_review_embed(
            vote_emoji,
            event_embed,
            comment.as_ref().map(|m| m.content.as_str()),
        );

        match dm
            .send_message(&self.client.http, CreateMessage::new().embed(embed))
            .await
        {
            Ok(_) => {
                record_dm_review_sent(MetricResult::Ok);
                if let Some(comment) = comment {
                    self.add_reaction_to_message(&comment, '✅').await;
                }
            }
            Err(e) => {
                record_dm_review_sent(MetricResult::Error);
                error!("Failed to send review DM to {}: {}", dm.recipient.name, e);
            }
        }
    }

    fn create_user_review_embed(
        vote_emoji: &EmojiConfig,
        event_embed: Embed,
        comment: Option<&str>,
    ) -> CreateEmbed {
        match comment {
            None => CreateEmbed::from(event_embed).field("Voto", vote_emoji.to_string(), true),
            Some(comment) => CreateEmbed::from(event_embed)
                .field("Voto", vote_emoji.to_string(), true)
                .field("Comentários", comment, true),
        }
    }

    /// Sends a review DM identical in format to `send_user_review_in_dm`, but with an
    /// explicit comment string instead of reading the user's last DM message.
    /// Used to backfill historical reviews that predate (or bypassed) the normal
    /// reaction -> DM flow.
    pub async fn send_backfill_review(
        &self,
        user_id: UserId,
        event_url: &str,
        vote_emoji: &EmojiConfig,
        comment: Option<&str>,
        channel_ids: &[ChannelId],
    ) -> Result<bool, ()> {
        let mut event_embed = None;

        for channel_id in channel_ids {
            if let Some(found) = self.find_event_embed(*channel_id, event_url).await {
                event_embed = Some(found);
                break;
            }
        }

        let Some(event_embed) = event_embed else {
            warn!("Could not find event message for url '{}'", event_url);
            return Ok(false);
        };

        let dm = match user_id.create_dm_channel(&self.client.http).await {
            Ok(dm) => dm,
            Err(e) => {
                error!("Couldn't create DM channel for user '{}': {}", user_id, e);
                return Err(());
            }
        };

        match self.is_event_sent_in_dm(event_url, &dm).await {
            Ok(true) => {
                warn!("Event already sent to user {}", user_id);
                return Ok(false);
            }
            Err(_) => return Err(()),
            Ok(false) => {}
        }

        let embed = Self::create_user_review_embed(vote_emoji, event_embed, comment);

        match dm
            .send_message(&self.client.http, CreateMessage::new().embed(embed))
            .await
        {
            Ok(_) => {
                record_dm_review_sent(MetricResult::Ok);
                info!("Backfilled review for event '{}'", event_url);
                Ok(true)
            }
            Err(e) => {
                record_dm_review_sent(MetricResult::Error);
                error!("Failed to send backfill review DM to {}: {}", user_id, e);
                Err(())
            }
        }
    }

    async fn find_event_embed(&self, channel_id: ChannelId, event_url: &str) -> Option<Embed> {
        let guild = self.get_guild(channel_id).await;
        let threads = self.get_channel_threads(&guild, channel_id).await;

        for thread in threads {
            let messages = self.get_all_messages(thread.id).await;

            if let Some(message) = messages.into_iter().find(|m| {
                m.embeds.first().and_then(|e| e.url.clone()).as_deref() == Some(event_url)
            }) {
                return message.embeds.into_iter().next();
            }
        }

        None
    }

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

    async fn is_event_sent_in_dm(
        &self,
        event_url: &str,
        dm: &PrivateChannel,
    ) -> Result<bool, serenity::Error> {
        let mut last_message_id: Option<MessageId> = None;
        let mut searched_all_dms = false;

        while !searched_all_dms {
            let mut filter = GetMessages::default();

            if let Some(last_message_id) = last_message_id {
                filter = filter.before(last_message_id)
            }

            let messages_iter = dm.messages(&self.client.http, filter).await.map_err(|e| {
                error!(
                    "Failed to fetch DM messages for '{}': {}",
                    dm.recipient.name, e
                );
                e
            })?;

            if messages_iter.iter().any(|msg| {
                msg.embeds
                    .first()
                    .and_then(|embed| embed.url.clone())
                    .unwrap_or_default()
                    == event_url
            }) {
                return Ok(true);
            }

            match messages_iter.first() {
                None => {
                    searched_all_dms = true;
                }
                Some(last_message) => last_message_id = Some(last_message.id),
            }
        }

        Ok(false)
    }

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

    fn truncate_embed_description(description: String) -> String {
        Self::truncate(description, 4096)
    }

    fn truncate(description: String, length: usize) -> String {
        if description.len() > length {
            let mut embed_description = description.clone();

            embed_description.truncate(length);

            format!("{embed_description}...")
        } else {
            description
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
        let has_no_user_reactions = DiscordAPI::has_someone_reacted(&reaction);

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
        let has_no_user_reactions = DiscordAPI::has_someone_reacted(&reaction);

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
        let has_no_user_reactions = DiscordAPI::has_someone_reacted(&reaction);

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
