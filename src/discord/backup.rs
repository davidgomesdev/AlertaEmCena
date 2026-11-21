use crate::config::model::EmojiConfig;
use crate::discord::api::DiscordAPI;
use serde::Serialize;
use serenity::all::{Embed, GetMessages, Message, MessageType, UserId};
use tracing::{error, info, instrument};

#[instrument(skip(discord))]
pub async fn backup_user_votes(discord: &DiscordAPI, user_id: UserId) -> Option<Vec<VoteRecord>> {
    let dm_channel = user_id.create_dm_channel(&discord.client.http).await;

    if let Err(err) = dm_channel {
        error!("Failed to create DM channel! Error: {}", err);
        return None;
    }

    let messages = dm_channel
        .unwrap()
        .messages(&discord.client.http, GetMessages::new())
        .await;

    if let Err(err) = messages {
        error!("Failed to get messages from DM channel! Error: {}", err);
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
    let user_vote = parse_user_vote(embed)?;

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

/// Reads the vote out of a review embed: the "Voto" field if present, otherwise the
/// bold `**Voto:** ` line in the description (older reviews).
pub fn parse_user_vote(embed: &Embed) -> Option<UserVote> {
    let embed_fields = &embed.fields;

    match embed_fields.iter().find(|field| field.name == "Voto") {
        Some(vote) => {
            let comments = embed_fields
                .iter()
                .find(|field| field.name == "Comentários")
                .map(|comment_field| comment_field.value.clone());

            Some(UserVote {
                vote: vote.value.clone(),
                comments,
            })
        }
        None => {
            // Fallback for embed-less reviews (backwards compatibility)
            let description = embed.description.as_deref().unwrap_or_default();
            let vote = description
                .lines()
                .find(|line| line.starts_with("**Voto:** "))
                .map(|line| line.replace("**Voto:** ", "").trim().to_string());
            let comments = description
                .lines()
                .find(|line| line.starts_with("**Comentários:** "))
                .map(|line| line.replace("**Comentários:** ", "").trim().to_string());

            let Some(vote) = vote else {
                error!("No vote found in description on an embed-less review!");
                return None;
            };

            Some(UserVote { vote, comments })
        }
    }
}

/// Goes through every message in the user's DM (paginating past Discord's 100 limit)
/// and counts how many votes each rating got. Index 0 is the worst rating.
#[instrument(skip(discord, vote_emojis))]
pub async fn count_user_votes(
    discord: &DiscordAPI,
    user_id: UserId,
    vote_emojis: &[EmojiConfig; 5],
) -> Option<[u32; 5]> {
    let dm_channel = match user_id.create_dm_channel(&discord.client.http).await {
        Ok(channel) => channel,
        Err(err) => {
            error!("Failed to create DM channel! Error: {}", err);
            return None;
        }
    };

    let mut votes: Vec<UserVote> = Vec::new();
    let mut before = None;

    loop {
        let mut builder = GetMessages::new().limit(100);
        if let Some(id) = before {
            builder = builder.before(id);
        }

        let page = match dm_channel.messages(&discord.client.http, builder).await {
            Ok(page) => page,
            Err(err) => {
                error!("Failed to get messages from DM channel! Error: {}", err);
                return None;
            }
        };

        let Some(oldest) = page.iter().min_by_key(|message| message.id) else {
            break;
        };
        before = Some(oldest.id);

        votes.extend(
            page.iter()
                .filter(|message| {
                    message.author.id == discord.own_user.id && message.kind == MessageType::Regular
                })
                .filter_map(|message| message.embeds.first())
                .filter_map(parse_user_vote),
        );
    }

    info!("Found {} votes", votes.len());

    Some(tally_votes(
        votes.iter().map(|vote| vote.vote.as_str()),
        vote_emojis,
    ))
}

/// Counts votes per rating; votes not matching any voting emoji are ignored.
pub fn tally_votes<'a>(
    votes: impl IntoIterator<Item = &'a str>,
    vote_emojis: &[EmojiConfig; 5],
) -> [u32; 5] {
    let mut counts = [0u32; 5];

    for vote in votes {
        match vote_emojis
            .iter()
            .position(|emoji| emoji.to_string() == vote.trim())
        {
            Some(index) => counts[index] += 1,
            None => error!("Unrecognized vote value: {}", vote),
        }
    }

    counts
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

#[cfg(test)]
mod tests {
    use super::*;

    fn emojis() -> [EmojiConfig; 5] {
        std::array::from_fn(|i| EmojiConfig {
            id: i as u64 + 1,
            name: format!("e{}", i + 1),
        })
    }

    fn embed(json: serde_json::Value) -> Embed {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn parses_vote_from_field() {
        let e = embed(serde_json::json!({
            "description": "desc",
            "fields": [{"name": "Voto", "value": "<:e2:2>", "inline": true}]
        }));

        assert_eq!(parse_user_vote(&e).unwrap().vote, "<:e2:2>");
    }

    #[test]
    fn parses_vote_from_bold_description_when_field_missing() {
        let e = embed(serde_json::json!({
            "description": "text\n**Voto:** <:e4:4>\n**Comentários:** nice",
        }));

        let vote = parse_user_vote(&e).unwrap();
        assert_eq!(vote.vote, "<:e4:4>");
        assert_eq!(vote.comments.as_deref(), Some("nice"));
    }

    #[test]
    fn no_vote_anywhere_yields_none() {
        assert!(parse_user_vote(&embed(serde_json::json!({"description": "x"}))).is_none());
    }

    #[test]
    fn tallies_votes_per_rating_including_zeros() {
        let counts = tally_votes(
            ["<:e1:1>", "<:e1:1>", "<:e2:2>", " <:e5:5> ", "unknown"],
            &emojis(),
        );

        assert_eq!(counts, [2, 1, 0, 0, 1]);
    }
}
