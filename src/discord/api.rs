use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use futures::{StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::{
    Colour, CreateEmbedAuthor, CurrentUser, Embed, GatewayIntents, GetMessages, Message, MessageId, PrivateChannel, ReactionType, User,
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

    pub async fn add_reaction_to_message(&self, message: &Message, emoji_char: char) {
        message
            .react(&self.client.http, ReactionType::from(emoji_char))
            .await
            .unwrap();
    }

    #[instrument(skip(self, message, emoji_char), fields(event = %message.embeds.first().map(|embed| embed.title.clone().unwrap()).unwrap_or_default()
    ))]
    pub async fn tag_save_for_later_reactions(&self, message: &mut Message, emoji_char: char) {
        let user_ids_that_reacted: Vec<String> = message
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
                    .collect()
            })
            .expect("Couldn't get users that reacted!");

        debug!(
            "Found '{:?}' users that reacted to the new message",
            user_ids_that_reacted
        );

        let mut users_already_in_list = Vec::new();

        if let Some(user_ids) = USER_MENTION_REGEX.captures(&message.content) {
            for user_id in user_ids.iter().skip(1).flatten() {
                users_already_in_list.push(user_id.as_str());
            }
        }

        let new_users = user_ids_that_reacted
            .into_iter()
            .map(|user_id| user_id.to_string())
            .filter(|user| !users_already_in_list.contains(&&**user))
            .map(|user| format!("<@{}>", user))
            .collect::<Vec<String>>()
            .join(" ");

        if new_users.is_empty() {
            debug!("No new users save for later");
            return;
        }

        info!("Found NEW users '{}' that saved for later", new_users);

        message
            .edit(
                &self.client.http,
                EditMessage::new()
                    .content(format!("Interessados: {} {}", message.content, new_users)),
            )
            .await
            .expect("Failed to edit message!");
    }

    pub async fn send_privately_users_vote(
        &self,
        event_message: &Message,
        voting_emojis: [EmojiConfig; 5],
    ) {
        let mut event_embed = event_message.embeds.first().cloned().unwrap();
        let event_url = event_embed.url.clone();

        if event_url.is_none() {
            warn!("Event has no URL!");
            return;
        }

        let mut users_votes: [Vec<User>; 5] = [vec![], vec![], vec![], vec![], vec![]];
        let own_user = self
            .client
            .http
            .get_current_user()
            .await
            .map(|user| user.id)
            .unwrap_or_default();

        for (index, voting_emoji) in voting_emojis.iter().enumerate() {
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
                if user.id == own_user || user.bot {
                    continue
                }

                users_votes[index].push(user);
            }
        }

        if users_votes.is_empty() {
            return
        }

        let event_url = event_url.unwrap();

        event_embed.fields = Vec::new();

        for (vote, users) in users_votes.iter().enumerate() {
            for user in users {
                match user.create_dm_channel(&self.client.http).await {
                    Ok(dm) => {
                        debug!("Found user {} with vote {}", user.id, vote+1);

                        if !self.is_event_sent_in_dm(&event_url, &dm).await {
                            self.send_user_vote_in_dm(&voting_emojis[vote], &event_embed, &dm)
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

    #[instrument(skip(self, vote_emoji, event_embed, dm), fields(user_name = %dm.recipient.name.to_string(), vote = %vote_emoji.name.to_string(), event_url = event_embed.url))]
    async fn send_user_vote_in_dm(
        &self,
        vote_emoji: &EmojiConfig,
        event_embed: &Embed,
        dm: &PrivateChannel,
    ) {
        info!("Sending vote");

        let embed = CreateEmbed::from(event_embed.clone()).description(format!(
            "{}\n**Voto:** {}",
            event_embed.description.clone().unwrap(),
            vote_emoji
        ));

        dm.send_message(&self.client.http, CreateMessage::new().embed(embed))
            .await
            .expect("Failed to send message");
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
    pub async fn delete_all_messages(&self, channel_id: ChannelId) {
        let messages = channel_id
            .messages_iter(&self.client.http)
            .try_collect::<Vec<Message>>()
            .await
            .expect("Failed to fetch messages");

        for chunk in messages.chunks(100) {
            debug!("Deleting {} messages", chunk.len());
            channel_id
                .delete_messages(&self.client.http, chunk)
                .await
                .expect("Failed to delete messages");
        }
    }
}
