use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::api::*;
use alertaemcena::config::env_loader::load_config;
use alertaemcena::config::model::{Config, DebugConfig, EmojiConfig};
use alertaemcena::discord::api::{DiscordAPI, EventsThread};
use alertaemcena::discord::backup::{backup_user_votes, VoteRecord};
use alertaemcena::tracing::setup_loki;
use chrono::Utc;
use futures::{future, TryFutureExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use serenity::all::{
    ChannelId, GuildChannel, MessageType, UserId,
};
use std::collections::BTreeMap;
use std::process::exit;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, instrument, trace, warn};

lazy_static! {
    pub static ref SAVE_FOR_LATER_EMOJI: char = '🔖';
}

#[tokio::main]
async fn main() {
    let loki_controller = setup_loki().await;

    {
        let _shutdown_hook = ShutdownHook;

        let config = load_config();

        debug!("Loaded {:?}", config);

        let discord = DiscordAPI::default().await;

        if config.debug_config.clear_channel {
            discord.delete_all_messages(&config.teatro_channel_id).await;
            discord.delete_all_messages(&config.artes_channel_id).await;

            if config.debug_config.exit_after_clearing {
                exit(0)
            }
        }

        let mut users_to_backup = Vec::new();

        if !config.debug_config.skip_artes {
            run(&config, &discord, Category::Artes, config.artes_channel_id)
                .await
                .iter()
                .for_each(|u| {
                    users_to_backup.push(*u);
                })
        }

        run(
            &config,
            &discord,
            Category::Teatro,
            config.teatro_channel_id,
        )
        .await
        .iter()
        .for_each(|u| {
            if !users_to_backup.contains(u) {
                users_to_backup.push(*u);
            }
        });

        backup_votes(&discord, users_to_backup).await;
    }

    if let Some((controller, join_handle)) = loki_controller {
        controller.shutdown().await;
        join_handle.await.expect("Failed joining Loki task");
    }
}

#[instrument(skip(config, discord, channel_id))]
async fn run(
    config: &Config,
    discord: &DiscordAPI,
    category: Category,
    channel_id: ChannelId,
) -> Vec<UserId> {
    let guild = discord.get_guild(channel_id).await;
    let threads = discord.get_channel_threads(&guild, channel_id).await;
    let mut users_with_reactions = Vec::new();

    if !config.debug_config.skip_feature_reactions {
        users_with_reactions =
            handle_reaction_features(discord, threads, &config.voting_emojis).await;
    }

    info!("Handled reaction features");

    if !config.gather_new_events {
        info!("Set to not gather new events");
        return users_with_reactions;
    }

    let events =
        AgendaCulturalAPI::get_events_by_month(&category, config.debug_config.event_limit).await;

    if let Err(err) = events {
        error!("Failed getting events. Reason: {:?}", err);

        return users_with_reactions;
    }

    let events = events.unwrap();

    if events.is_empty() {
        error!("No events found");
        return users_with_reactions;
    }

    let new_events = filter_new_events_by_thread(discord, &guild, events, channel_id).await;

    info!("Filtered new events");

    send_new_events(
        discord,
        new_events,
        &config.debug_config,
        &config.voting_emojis,
    )
    .await;

    info!("Finished sending new events for {}", category);

    users_with_reactions
}

#[instrument(skip(discord))]
pub async fn backup_votes(discord: &DiscordAPI, vec: Vec<UserId>) {
    let vote_backups_folder = "vote_backups/";
    let vote_backup_file_path = format!(
        "{}{}.json",
        vote_backups_folder,
        Utc::now().format("%Y_%m_%d")
    );

    fs::try_exists(vote_backups_folder)
        .and_then(|exists| async move {
            if exists {
                Ok(())
            } else {
                fs::create_dir(vote_backups_folder).await
            }
        })
        .unwrap_or_else(|e| {
            error!("Failed to create vote backups folder! Error: {}", e);
        })
        .await;

    match fs::try_exists(vote_backup_file_path.clone()).await {
        Ok(exists) => {
            if exists {
                info!("Vote backup file already exists for today, skipping backup");
                return;
            }
        }
        Err(e) => {
            error!("Failed to check if vote backup file exists! Error: {}", e);
            return;
        }
    }

    let user_votes: Vec<VoteRecord> = future::join_all(
        vec.iter()
            .map(|user_id| backup_user_votes(discord, *user_id)),
    )
    .await
    .into_iter()
    .flatten()
    .concat();

    let backup_votes_file = File::create(&vote_backup_file_path).await;

    if let Err(err) = backup_votes_file {
        error!(
            "Failed to create vote backup file at {}! Error: {}",
            vote_backup_file_path,
            err
        );
        return;
    }

    let write_return = backup_votes_file.unwrap()
        .write_all(&serde_json::to_vec_pretty(&user_votes).expect("Failed to serialize user votes"))
        .await;

    if let Err(e) = write_return {
        error!("Failed to write vote backup file! Error: {}", e);
        return;
    }

    info!("Vote backup file written to {}", vote_backup_file_path);
}

/// Returns users who have used reaction features in the given threads
#[instrument(skip(discord, threads, vote_emojis))]
async fn handle_reaction_features(
    discord: &DiscordAPI,
    threads: Vec<GuildChannel>,
    vote_emojis: &[EmojiConfig; 5],
) -> Vec<UserId> {
    let mut users_with_reactions = Vec::new();

    for thread in threads {
        if thread.thread_metadata.expect("Should be a thread!").locked {
            trace!("Ignoring locked thread (probably out-of-date)");
            continue;
        }

        let messages = discord
            .get_all_messages(thread.id)
            .await
            .expect("Failed to get messages");

        info!(
            "Tagging save for later and sending votes in DM (on thread '{}' with {} messages)",
            thread.name,
            messages.len()
        );

        for mut message in messages {
            if message.author != *discord.own_user {
                debug!(
                    "Ignoring message from a different user {}",
                    message.author.id
                );
                continue;
            }

            if message.kind != MessageType::Regular {
                trace!("Ignoring non-regular message (id={})", message.id);
                continue;
            }

            if message.embeds.is_empty() {
                warn!(
                    "Found message without embed (id={}; content={})",
                    message.id, message.content
                );
                continue;
            }

            discord
                .tag_save_for_later_reactions(&mut message, *SAVE_FOR_LATER_EMOJI)
                .await;

            discord
                .send_privately_users_review(&message, vote_emojis)
                .await
                .iter()
                .for_each(|u| {
                    if !users_with_reactions.contains(u) {
                        users_with_reactions.push(*u);
                    }
                });
        }
    }

    users_with_reactions
}

#[instrument(skip(discord, new_events, emojis, debug_config), fields(new_events_count = %new_events.len()
))]
async fn send_new_events(
    discord: &DiscordAPI,
    new_events: BTreeMap<EventsThread, Vec<Event>>,
    debug_config: &DebugConfig,
    emojis: &[EmojiConfig; 5],
) {
    if new_events.is_empty() {
        info!("No new events to send");
        return;
    }

    if debug_config.skip_sending {
        info!("Skipping sending events");
        return;
    }

    for (thread, events) in new_events {
        info!(
            "Found {} new events for thread '{}'",
            events.len(),
            thread
                .thread_id
                .name(&discord.client.http)
                .await
                .unwrap_or_default()
        );

        for event in events {
            let message = discord.send_event(thread.thread_id, event).await;

            if debug_config.skip_feature_reactions {
                info!("Skipping feature reactions");
                continue;
            }

            add_feature_reactions(discord, &message, emojis, *SAVE_FOR_LATER_EMOJI).await;
        }
    }
}

struct ShutdownHook;

impl Drop for ShutdownHook {
    fn drop(&mut self) {
        info!("App finished")
    }
}
