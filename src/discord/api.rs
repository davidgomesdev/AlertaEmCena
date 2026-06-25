use crate::agenda_cultural::api::AgendaCulturalAPI;
use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use crate::metrics::{record_dm_review_rewrite, record_dm_review_sent, MetricResult};
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
use std::collections::HashMap;
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
const PROCESSED_COMMENT_EMOJI: char = '✅';

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

const FALLBACK_EMBED_COLOR: u32 = 0x005eeb;

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

        let title = event.title.clone();
        let embed = Self::build_event_embed(event, ticket_shop_url, ticket_shop_icon_url).await;

        let message_builder = CreateMessage::new().add_embed(embed.clone());

        channel_id
            .send_message(&self.client.http, message_builder)
            .await
            .map_err(|err| {
                error!("Failed sending event '{}' due to '{}'", title, err);
                DiscordError::Api
            })
    }

    async fn build_event_embed(
        event: Event,
        ticket_shop_url: Option<String>,
        ticket_shop_icon_url: &str,
    ) -> CreateEmbed {
        let mut description = event.details.description;

        if event.is_for_children {
            description = format!("{}\n\n{CHILDREN_LABEL}", description.clone());
        }

        let mut author = CreateEmbedAuthor::new(&event.venue);

        if let Some(ticket_shop_url) = ticket_shop_url {
            author = author.url(ticket_shop_url).icon_url(ticket_shop_icon_url);
        }

        let embed_description = Self::truncate_embed_description(description);
        let color = Self::get_image_dominant_color(&event.details.image_url)
            .await
            .unwrap_or_else(|| Colour::new(FALLBACK_EMBED_COLOR));

        CreateEmbed::new()
            .title(event.title)
            .url(event.link)
            .description(embed_description)
            .author(author)
            .color(color)
            .field("Datas", event.occurring_at.dates, true)
            .image(event.details.image_url)
    }

    pub async fn get_image_dominant_color(image_url: &str) -> Option<Colour> {
        let bytes = reqwest::get(image_url)
            .await
            .map_err(|e| warn!("Failed fetching event image '{}': {}", image_url, e))
            .ok()?
            .bytes()
            .await
            .map_err(|e| warn!("Failed reading event image bytes '{}': {}", image_url, e))
            .ok()?;

        let rgba = image::load_from_memory(&bytes)
            .map_err(|e| warn!("Failed decoding event image '{}': {}", image_url, e))
            .ok()?
            .to_rgba8();

        let palette =
            color_thief::get_palette(rgba.as_raw(), color_thief::ColorFormat::Rgba, 10, 8)
                .map_err(|e| warn!("Failed extracting palette for '{}': {:?}", image_url, e))
                .ok()?;

        let dominant = palette
            .iter()
            .find(|c| Self::is_colorful(c.r, c.g, c.b))
            .or_else(|| palette.first())?;

        Some(Colour::from_rgb(dominant.r, dominant.g, dominant.b))
    }

    /// Excludes near-grayscale colors so a vivid palette entry wins over a washed-out background.
    fn is_colorful(r: u8, g: u8, b: u8) -> bool {
        const SATURATION_THRESHOLD: u8 = 30;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);

        max - min > SATURATION_THRESHOLD
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

    fn message_has_bot_reaction(reactions: &[MessageReaction], emoji_char: &str) -> bool {
        reactions.iter().any(|reaction| {
            if let Unicode(char) = &reaction.reaction_type {
                *char == emoji_char && reaction.me
            } else {
                false
            }
        })
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
                    self.add_reaction_to_message(&comment, PROCESSED_COMMENT_EMOJI)
                        .await;
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

    pub async fn send_backfill_review(
        &self,
        user_id: UserId,
        event_url: &str,
        vote_emoji: &EmojiConfig,
        comment: Option<&str>,
        venue_ticket_shop_url: &HashMap<String, String>,
        ticket_shop_icon_url: &str,
    ) -> Result<bool, ()> {
        let Some(event) = AgendaCulturalAPI::scrape_event(event_url).await else {
            warn!("Could not scrape event details for '{}'", event_url);
            return Ok(false);
        };

        let ticket_shop_url = venue_ticket_shop_url.get(&event.venue).cloned();

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

        let mut embed = Self::build_event_embed(event, ticket_shop_url, ticket_shop_icon_url)
            .await
            .field("Voto", vote_emoji.to_string(), true);

        if let Some(comment) = comment {
            embed = embed.field("Comentários", comment, true);
        }

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

    pub async fn rewrite_reviews_from_dm_replies(&self, user_id: UserId) -> usize {
        let dm = match user_id.create_dm_channel(&self.client.http).await {
            Ok(dm) => dm,
            Err(e) => {
                warn!("Couldn't create DM channel for user '{}': {}", user_id, e);
                return 0;
            }
        };

        let mut messages = match self.fetch_all_dm_messages(&dm).await {
            Ok(messages) => messages,
            Err(_) => return 0,
        };

        // fetch_all_dm_messages returns newest-to-oldest with no gaps or overlaps;
        // reverse to oldest-first so the latest reply to any given review embed is
        // always processed last.
        messages.reverse();

        let mut rewritten_count = 0;

        for reply in &messages {
            if !Self::is_message_a_rewrite_request(self.own_user.id, reply) {
                continue;
            }

            if self.rewrite_review_from_reply(&dm, reply).await {
                rewritten_count += 1;
            }
        }

        rewritten_count
    }

    fn is_message_a_rewrite_request(own_user_id: UserId, reply: &Message) -> bool {
        let is_a_user_message = reply.author.id != own_user_id;

        is_a_user_message
            && reply.referenced_message.as_ref().is_some_and(|referenced| {
                let is_a_reply_to_bot_message = referenced.author.id == own_user_id;
                let has_vote = referenced
                    .embeds
                    .first()
                    .is_some_and(|embed| embed.fields.iter().any(|field| field.name == "Voto"));

                is_a_reply_to_bot_message && has_vote
            })
            && !Self::message_has_bot_reaction(
                &reply.reactions,
                &PROCESSED_COMMENT_EMOJI.to_string(),
            )
    }

    async fn fetch_all_dm_messages(
        &self,
        dm: &PrivateChannel,
    ) -> Result<Vec<Message>, serenity::Error> {
        let mut all_messages = Vec::new();
        let mut last_message_id: Option<MessageId> = None;

        loop {
            let mut filter = GetMessages::default();

            if let Some(id) = last_message_id {
                filter = filter.before(id);
            }

            let page = dm.messages(&self.client.http, filter).await.map_err(|e| {
                error!(
                    "Failed to fetch DM messages for '{}': {}",
                    dm.recipient.name, e
                );
                e
            })?;

            match page.last() {
                None => break,
                Some(oldest_in_page) => last_message_id = Some(oldest_in_page.id),
            }

            all_messages.extend(page);
        }

        Ok(all_messages)
    }

    async fn rewrite_review_from_reply(&self, dm: &PrivateChannel, reply: &Message) -> bool {
        let referenced = reply
            .referenced_message
            .as_ref()
            .expect("should not have landed here");

        let embed = referenced
            .embeds
            .first()
            .expect("should not have landed here");

        let voto_value = embed
            .fields
            .iter()
            .find(|field| field.name == "Voto")
            .expect("should not have landed here")
            .value
            .clone();

        let mut fresh = match self.client.http.get_message(dm.id, referenced.id).await {
            Ok(message) => message,
            Err(e) => {
                error!(
                    "Failed to refetch review message {} for user {}: {}",
                    referenced.id, reply.author.id, e
                );
                return false;
            }
        };

        let Some(mut fresh_embed) = fresh.embeds.first().cloned() else {
            return false;
        };
        fresh_embed.fields = Vec::new();

        let new_embed = CreateEmbed::from(fresh_embed)
            .field("Voto", voto_value, true)
            .field("Comentários", reply.content.clone(), true);

        match fresh
            .edit(&self.client.http, EditMessage::new().embed(new_embed))
            .await
        {
            Ok(_) => {
                record_dm_review_rewrite(MetricResult::Ok);
                self.add_reaction_to_message(reply, PROCESSED_COMMENT_EMOJI)
                    .await;
                true
            }
            Err(e) => {
                record_dm_review_rewrite(MetricResult::Error);
                error!(
                    "Failed to rewrite review message {} for user {}: {}",
                    referenced.id, reply.author.id, e
                );
                false
            }
        }
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

            match messages_iter.last() {
                None => {
                    searched_all_dms = true;
                }
                Some(oldest_in_page) => last_message_id = Some(oldest_in_page.id),
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

    const BOT_USER_ID: u64 = 1;
    const OTHER_USER_ID: u64 = 2;

    fn build_message(author_id: u64, referenced_message: Option<serde_json::Value>) -> Message {
        let json = serde_json::json!({
            "id": "1",
            "channel_id": "1",
            "author": { "id": author_id.to_string(), "username": "user" },
            "content": "",
            "timestamp": "2024-01-01T00:00:00.000000+00:00",
            "tts": false,
            "mention_everyone": false,
            "mentions": [],
            "mention_roles": [],
            "attachments": [],
            "embeds": [],
            "pinned": false,
            "type": 0,
            "referenced_message": referenced_message,
        });

        serde_json::from_value(json).expect("Failed to build test message")
    }

    fn build_review_embed(has_vote_field: bool) -> serde_json::Value {
        let fields = if has_vote_field {
            serde_json::json!([{ "name": "Voto", "value": "🟩", "inline": true }])
        } else {
            serde_json::json!([])
        };

        serde_json::json!({ "fields": fields })
    }

    fn build_reply_to_bot_review(
        reply_author_id: u64,
        has_vote_field: bool,
        already_processed: bool,
    ) -> Message {
        let mut referenced = build_message(BOT_USER_ID, None);
        referenced.embeds = vec![serde_json::from_value(build_review_embed(has_vote_field))
            .expect("Failed to build test embed")];

        let mut reply = build_message(reply_author_id, None);
        reply.referenced_message = Some(Box::new(referenced));

        if already_processed {
            reply.reactions = serde_json::from_str(
                r#"
            [{
              "count": 1,
              "count_details": { "burst": 0, "normal": 1 },
              "me": true,
              "me_burst": false,
              "emoji": { "id": null, "name": "✅" },
              "burst_colors": []
            }]
            "#,
            )
            .expect("Failed to build test reaction");
        }

        reply
    }

    #[test_log::test]
    fn when_reply_is_from_bot_itself_should_return_false() {
        let reply = build_reply_to_bot_review(BOT_USER_ID, true, false);

        assert!(!DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_message_is_not_a_reply_should_return_false() {
        let reply = build_message(OTHER_USER_ID, None);

        assert!(!DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_reply_is_not_to_a_bot_message_should_return_false() {
        let referenced = build_message(OTHER_USER_ID, None);
        let mut reply = build_message(OTHER_USER_ID, None);
        reply.referenced_message = Some(Box::new(referenced));

        assert!(!DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_referenced_bot_message_has_no_vote_field_should_return_false() {
        let reply = build_reply_to_bot_review(OTHER_USER_ID, false, false);

        assert!(!DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_reply_already_processed_should_return_false() {
        let reply = build_reply_to_bot_review(OTHER_USER_ID, true, true);

        assert!(!DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_reply_is_a_valid_unprocessed_rewrite_request_should_return_true() {
        let reply = build_reply_to_bot_review(OTHER_USER_ID, true, false);

        assert!(DiscordAPI::is_message_a_rewrite_request(
            UserId::from(BOT_USER_ID),
            &reply
        ));
    }

    #[test_log::test]
    fn when_bot_has_reacted_with_emoji_should_return_true() {
        let reactions: Vec<MessageReaction> = serde_json::from_str(
            r#"
        [{
          "count": 1,
          "count_details": { "burst": 0, "normal": 1 },
          "me": true,
          "me_burst": false,
          "emoji": { "id": null, "name": "✅" },
          "burst_colors": []
        }]
        "#,
        )
        .unwrap();

        assert!(DiscordAPI::message_has_bot_reaction(&reactions, "✅"));
    }

    #[test_log::test]
    fn when_bot_has_not_reacted_with_emoji_should_return_false() {
        let reactions: Vec<MessageReaction> = serde_json::from_str(
            r#"
        [{
          "count": 1,
          "count_details": { "burst": 0, "normal": 1 },
          "me": false,
          "me_burst": false,
          "emoji": { "id": null, "name": "✅" },
          "burst_colors": []
        }]
        "#,
        )
        .unwrap();

        assert!(!DiscordAPI::message_has_bot_reaction(&reactions, "✅"));
    }

    #[test_log::test]
    fn when_no_matching_emoji_reaction_should_return_false() {
        let reactions: Vec<MessageReaction> = serde_json::from_str(
            r#"
        [{
          "count": 1,
          "count_details": { "burst": 0, "normal": 1 },
          "me": true,
          "me_burst": false,
          "emoji": { "id": null, "name": "🔖" },
          "burst_colors": []
        }]
        "#,
        )
        .unwrap();

        assert!(!DiscordAPI::message_has_bot_reaction(&reactions, "✅"));
    }

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
