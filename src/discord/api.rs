use crate::agenda_cultural::model::Event;
use futures::StreamExt;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::{Colour, CreateEmbedAuthor, Embed, GatewayIntents, Message};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::SerenityError;
use serenity::Client;
use std::env;

const AUTHOR_NAME: &str = "AlertaEmCena";

lazy_static! {
    static ref REMOVE_YEAR: Regex = Regex::new(r"\b\d{4}\b").unwrap();
}

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
                .expect("Error creating client"),
        }
    }

    pub async fn send_event(&self, channel_id: ChannelId, event: Event) {
        let message_builder = CreateMessage::new().add_embed(
            CreateEmbed::new()
                .title(event.title.clone())
                .url(event.link.clone())
                .description(event.details.description.clone())
                .author(CreateEmbedAuthor::new(AUTHOR_NAME))
                .color(Colour::new(0x005eeb))
                .field(
                    "Datas",
                    REMOVE_YEAR
                        .replace_all(&event.occurring_at.dates, "")
                        .to_string(),
                    true,
                )
                .field("Onde", event.venue.clone(), true)
                .image(event.details.image_url.clone()),
        );

        channel_id
            .send_message(&self.client.http, message_builder)
            .await
            .expect("Failed to send message");
    }

    pub async fn get_event_urls_sent(&self, channel_id: ChannelId) -> Vec<String> {
        channel_id
            .messages_iter(&self.client.http)
            .map::<_, fn(_) -> Vec<Embed>>(|message: Result<Message, SerenityError>| {
                message.unwrap().embeds
            })
            .concat()
            .await
            .iter()
            .map(|embed| embed.url.clone())
            .flatten()
            .collect()
    }
}
