use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use futures::{StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::{
    Colour, CreateEmbedAuthor, CurrentUser, Embed, GatewayIntents, GetMessages, Message, MessageId,
    PrivateChannel, ReactionType, User,
};
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::cache::Settings;
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;
use tracing::{debug, error, info, instrument, warn};

const AUTHOR_NAME: &str = "AlertaEmCena";

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

        let message_builder = CreateMessage::new().add_embed(
            CreateEmbed::new()
                .title(event.title.clone())
                .url(event.link.clone())
                .description(event.details.description.clone())
                .author(CreateEmbedAuthor::new(AUTHOR_NAME))
                .color(Colour::new(0x005eeb))
                .field("Datas", event.occurring_at.dates, true)
                .field("Onde", event.venue.clone(), true)
                .image(event.details.image_url.clone()),
        );

        channel_id
            .send_message(&self.client.http, message_builder)
            .await
            .expect("Failed to send message")
    }

    #[instrument(skip(self, message, emoji))]
    pub async fn add_custom_reaction(&self, message: &Message, emoji: &EmojiConfig) {
        debug!("Adding reaction");

        match message
            .react(
                &self.client.http,
                ReactionType::Custom {
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
                debug!("Successfully added '{}' reaction", emoji.name);
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

        match react_result {
            Err(e) => {
                let msg = &format!("Failed to add reaction {} to message", emoji_char);
                error!(
                    msg,
                    error = %e
                );
            }
            _ => {}
        };
    }

    #[instrument(skip(self, message, vote_emojis), fields(event = %message.embeds.first().map(|embed| embed.url.clone().unwrap()).unwrap_or_default()
    ))]
    pub async fn tag_save_for_later_reactions(
        &self,
        message: &mut Message,
        emoji_char: char,
        vote_emojis: &[EmojiConfig; 5],
    ) {
        let users_that_voted: Vec<String> = self
            .get_user_votes(message, vote_emojis)
            .await
            .iter()
            .flatten()
            .map(|user| user.id.to_string())
            .collect();
        let saved_for_later_user_ids: Vec<String> = message
            .reaction_users(
                &self.client.http,
                ReactionType::from(emoji_char),
                None,
                None,
            )
            .await
            .map(|users| {
                users
                    .into_iter()
                    .map(|user| user.id.to_string())
                    .filter(|user_id| *user_id != self.own_user.id.to_string())
                    .filter(|user_id| !users_that_voted.contains(user_id))
                    .collect()
            })
            .expect("Couldn't get users that reacted!");

        if saved_for_later_user_ids.is_empty() && message.content.is_empty() {
            debug!("No users saved for later");
            return;
        }

        let mentions = saved_for_later_user_ids
            .iter()
            .map(|user_id| format!("<@{}>", user_id))
            .collect::<Vec<String>>()
            .join(" ");
        let message_content = format!("Interessados: {}", mentions);

        if message_content == message.content {
            return;
        }

        info!("Saved for later changed to '{}'", mentions);

        message
            .edit(
                &self.client.http,
                EditMessage::new().content(message_content),
            )
            .await
            .expect("Failed to edit message!");
    }

    pub async fn send_privately_users_vote(
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
            return;
        }

        let event_url = event_url.unwrap();

        event_embed.fields = Vec::new();

        for (vote, users) in users_votes.iter().enumerate() {
            for user in users.iter().filter(|user| !user.bot) {
                match user.create_dm_channel(&self.client.http).await {
                    Ok(dm) => {
                        debug!("Found user {} with vote {}", user.id, vote + 1);

                        if !self.is_event_sent_in_dm(&event_url, &dm).await {
                            self.send_user_review_in_dm(
                                &vote_emojis[vote],
                                event_embed.clone(),
                                &dm,
                            )
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
        }
    }

    async fn get_user_votes(
        &self,
        event_message: &Message,
        vote_emojis: &[EmojiConfig; 5],
    ) -> [Vec<User>; 5] {
        let mut users_votes: [Vec<User>; 5] = [vec![], vec![], vec![], vec![], vec![]];
        let own_user = self
            .client
            .http
            .get_current_user()
            .await
            .map(|user| user.id)
            .unwrap_or_default();

        for (index, voting_emoji) in vote_emojis.iter().enumerate() {
            let users_that_reacted: Vec<User> = event_message
                .reaction_users(
                    &self.client.http,
                    ReactionType::Custom {
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
                if user.id == own_user {
                    continue;
                }

                users_votes[index].push(user);
            }
        }

        users_votes
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
}
