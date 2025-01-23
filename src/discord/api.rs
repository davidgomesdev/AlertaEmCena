use crate::agenda_cultural::model::Event;
use crate::config::model::EmojiConfig;
use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    Colour, CreateEmbedAuthor, Embed, GatewayIntents, Message, ReactionType,
};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;
use tracing::{debug, error, info, instrument};

const AUTHOR_NAME: &str = "AlertaEmCena";

pub struct DiscordAPI {
    client: Client,
}

impl DiscordAPI {
    pub async fn default() -> Self {
        DiscordAPI::new(&env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set")).await
    }

    pub async fn new(token: &str) -> Self {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGE_REACTIONS;

        Self {
            client: Client::builder(token, intents)
                .await
                .expect("Error creating discord client"),
        }
    }

    #[instrument(skip(self, channel_id), fields(channel_id = %channel_id.to_string(), event = %event.title.to_string()))]
    pub async fn send_event(
        &self,
        channel_id: ChannelId,
        event: Event
    ) -> Message {
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
    pub async fn vote_message(&self, message: &Message, emoji: &EmojiConfig) {
        debug!("Adding vote");

        match message
            .react(
                &self.client.http,
                ReactionType::Custom {
                    animated: false,
                    id: emoji.id
                        .to_string()
                        .parse()
                        .expect("Invalid emoji ID format"),
                    name: Some(emoji.name.to_string()),
                },
            )
            .await
        {
            Ok(_) => {
                debug!("Successfully added '{}' vote reaction", emoji.name);
            }
            Err(err) => {
                error!(
                    "Failed to add '{}' ID {} vote reaction on own message: {:?}",
                    emoji.name, emoji.id, err
                );
            }
        }
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
