use crate::discord::api::DiscordAPI;
use serde::Serialize;
use serenity::all::{GetMessages, Message, MessageType, UserId};
use tracing::{error, info, instrument};

#[instrument(skip(discord))]
pub async fn backup_user_votes(discord: &DiscordAPI, user_id: UserId) -> Option<Vec<VoteRecord>> {
    let dm_channel = user_id.create_dm_channel(&discord.client.http).await;

    if dm_channel.is_err() {
        error!(
            "Failed to create DM channel! Error: {}",
            dm_channel.unwrap_err()
        );
        return None;
    }

    let messages = dm_channel
        .unwrap()
        .messages(&discord.client.http, GetMessages::new())
        .await;

    if messages.is_err() {
        error!(
            "Failed to get messages from DM channel! Error: {}",
            messages.unwrap_err()
        );
        return None;
    }

    let messages: Vec<VoteRecord> = messages
        .unwrap()
        .iter()
        .filter_map(|message| extract_vote(discord, user_id, message))
        .collect();

    info!("Found {} votes", messages.len());

    Some(messages)
}

#[instrument(skip_all)]
fn extract_vote(discord: &DiscordAPI, user_id: UserId, message: &Message) -> Option<VoteRecord> {
    if message.author.id != discord.own_user.id
        || message.kind != MessageType::Regular
        || message.embeds.is_empty()
    {
        return None;
    }

    let embed = &message.embeds[0];
    let description = embed.description.clone();

    if description.is_none() {
        error!("No description on event!");
        return None;
    }

    let description = description.unwrap();
    let embed_fields = &embed.fields;
    let user_vote = match embed_fields
        .iter()
        .find(|field| field.name == "Voto")
        .cloned()
    {
        Some(vote) => {
            let comments = embed_fields
                .iter()
                .find(|field| field.name == "Comentários")
                .map(|comment_field| comment_field.value.clone());

            UserVote {
                vote: vote.value,
                comments,
            }
        }
        None => {
            // Fallback for embed-less reviews (backwards compatibility)
            let vote = description
                .lines()
                .find(|line| line.starts_with("**Voto:** "))
                .map(|line| line.replace("**Voto:** ", "").trim().to_string());
            let comments = description
                .lines()
                .find(|line| line.starts_with("**Comentários:** "))
                .map(|line| line.replace("**Comentários:** ", "").trim().to_string());

            if vote.is_none() {
                error!("No vote found in description on an embed-less review!");
                return None;
            }

            UserVote {
                vote: vote.unwrap(),
                comments,
            }
        }
    };

    Some(VoteRecord {
        user_id,
        title: embed.title.clone().unwrap_or_else(|| {
            error!("No title on event");
            "No Title".to_string()
        }),
        url: embed.url.clone().unwrap_or_else(|| {
            error!("No URL on event");
            "No URL".to_string()
        }),
        description,
        user_vote,
    })
}

#[derive(Serialize, Debug)]
pub struct VoteRecord {
    pub user_id: UserId,
    pub title: String,
    pub url: String,
    pub description: String,
    pub user_vote: UserVote,
}

#[derive(Serialize, Debug)]
pub struct UserVote {
    pub vote: String,
    pub comments: Option<String>,
}
