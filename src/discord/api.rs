use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use futures::{Stream, StreamExt, TryStreamExt};
use serenity::all::{
    Colour, CreateEmbedAuthor, CurrentUser, Embed, GatewayIntents,
    Message, ReactionType, UserId,
};
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::cache::Settings;
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;
use tracing::{debug, error, info, instrument, warn};

const AUTHOR_NAME: &str = "AlertaEmCena";

pub struct DiscordAPI {
    client: Client,
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

        Self {
            client,
            own_user,
        }
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
        channel_id.messages_iter(&self.client.http).try_collect().await
    }

    pub async fn add_reaction_to_message(&self, message: &Message, emoji_char: char) {
        message
            .react(&self.client.http, ReactionType::from(emoji_char))
            .await
            .unwrap();
    }

    pub async fn tag_save_for_later_reactions(&self, message: &mut Message, emoji_char: char) {
        let user_ids: Vec<String> = message
            .reaction_users(
                &self.client.http,
                ReactionType::from(emoji_char),
                None,
                None,
            )
            .await.map(|users| users
                    .into_iter()
                    .map(|user| user.id.to_string())
                    .filter(|user_id| *user_id != self.own_user.id.to_string())
                    .collect())
            .expect("Couldn't get users that reacted!");

        debug!(
            "Found '{:?}' users that reacted to the new message",
            user_ids
        );

        match message.embeds.first() {
            None => {
                warn!("No embeds found!")
            }
            Some(embed) => {
                let embed_copy = embed.clone();
                let users_already_in_list: String = embed_copy
                    .fields
                    .into_iter()
                    .filter(|field| field.name == "Interessados")
                    .map(|embed| embed.value)
                    .next()
                    .map(|value| value.to_string())
                    .unwrap_or("".to_string());
                let new_users = user_ids
                    .iter()
                    .map(|user_id| user_id.to_string())
                    .filter(|user| !users_already_in_list.contains(user))
                    .map(|user| format!("<@{}>", user))
                    .collect::<Vec<String>>()
                    .join(" ");

                debug!("Found {} NEW users that reacted to message", new_users);

                message
                    .edit(
                        &self.client.http,
                        EditMessage::new().embed(CreateEmbed::from(embed.clone()).field(
                            "Interessados",
                            format!("{} {}", users_already_in_list, new_users),
                            false,
                        )),
                    )
                    .await
                    .expect("Failed to edit message!");
            }
        };
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

pub struct DiscordUser {
    user_id: UserId,
    pub username: String,
}

enum Vote {
    One,
    Two,
    Three,
    Four,
    Five,
}
