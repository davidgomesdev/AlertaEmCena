use alertaemcena::agenda_cultural::api::AgendaCulturalAPI;
use alertaemcena::agenda_cultural::model::{Category, Event};
use alertaemcena::api::*;
use alertaemcena::config::env_loader::load_config;
use alertaemcena::config::model::{Config, EmojiConfig};
use alertaemcena::discord::api::{DiscordAPI, EventsThread};
use alertaemcena::discord::backup::{backup_user_votes, VoteRecord};
use alertaemcena::metrics::{
    record_event_send_duration, record_event_sent, record_events_fetched,
    record_get_events_by_month_duration, record_pipeline_error, record_pipeline_run_duration,
    record_pipeline_run_duration_without_event_gather, record_reaction_processing_duration,
    record_vote_backup_duration, record_vote_backup_records, set_threads_active, MetricResult,
    PipelineErrorKind, PipelineStage,
};
use alertaemcena::tracing::setup_tracing;
use chrono::Utc;
use futures::{future, TryFutureExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use serenity::all::{ChannelId, GuildChannel, MessageType, UserId};
use std::collections::BTreeMap;
use std::process::exit;
use std::time::Instant;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, info_span, instrument, trace, warn, Instrument};

lazy_static! {
    pub static ref SAVE_FOR_LATER_EMOJI: char = '🔖';
}

#[tokio::main]
async fn main() {
    let tracing_handles = setup_tracing().await;

    {
        let _shutdown_hook = ShutdownHook;

        let root_span = info_span!("run");

        async move {
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
                    .instrument(info_span!("pipeline", category = "Artes"))
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
            .instrument(info_span!("pipeline", category = "Teatro"))
            .await
            .iter()
            .for_each(|u| {
                if !users_to_backup.contains(u) {
                    users_to_backup.push(*u);
                }
            });

            backup_votes(&discord, users_to_backup).await;
            info!("Starting app");
        }
        .instrument(root_span)
        .await;
    }

    tracing_handles.shutdown().await;
}

#[instrument(skip(config, discord, channel_id))]
async fn run(
    config: &Config,
    discord: &DiscordAPI,
    category: Category,
    channel_id: ChannelId,
) -> Vec<UserId> {
    let pipeline_started_at = Instant::now();
    let guild = discord.get_guild(channel_id).await;
    let threads = discord.get_channel_threads(&guild, channel_id).await;
    set_threads_active(&category, threads.len() as u64);
    let mut users_with_reactions = Vec::new();

    if !config.debug_config.skip_feature_reactions {
        let reaction_started_at = Instant::now();
        users_with_reactions =
            handle_reaction_features(discord, threads, &config.voting_emojis).await;
        record_reaction_processing_duration(&category, reaction_started_at.elapsed());
    }

    info!("Handled reaction features");

    if !config.gather_new_events {
        info!("Set to not gather new events");
        record_pipeline_run_duration_without_event_gather(&category, pipeline_started_at.elapsed());
        return users_with_reactions;
    }

    let get_events_started_at = Instant::now();
    let events =
        AgendaCulturalAPI::get_events_by_month(&category, config.debug_config.event_limit).await;
    record_get_events_by_month_duration(&category, get_events_started_at.elapsed());

    if let Err(err) = events {
        error!("Failed getting events. Reason: {:?}", err);
        record_pipeline_error(PipelineStage::FetchEvents, PipelineErrorKind::Api);
        record_pipeline_run_duration(&category, pipeline_started_at.elapsed());

        return users_with_reactions;
    }

    let events = events.unwrap();

    if events.is_empty() {
        error!("No events found");
        record_pipeline_error(PipelineStage::FetchEvents, PipelineErrorKind::EmptyResult);
        record_pipeline_run_duration(&category, pipeline_started_at.elapsed());
        return users_with_reactions;
    }

    let fetched_count: usize = events.values().map(|events| events.len()).sum();
    record_events_fetched(&category, fetched_count as u64);

    let new_events = filter_new_events_by_thread(discord, &guild, events, channel_id)
        .instrument(info_span!("filter_new_events"))
        .await;

    info!("Filtered new events");

    send_new_events(discord, new_events, config, &category).await;

    info!("Finished sending new events for {}", category);
    record_pipeline_run_duration(&category, pipeline_started_at.elapsed());

    users_with_reactions
}

#[instrument(skip(discord))]
pub async fn backup_votes(discord: &DiscordAPI, vec: Vec<UserId>) {
    let backup_started_at = Instant::now();
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
            record_pipeline_error(PipelineStage::BackupVotes, PipelineErrorKind::Io);
        })
        .await;

    match fs::try_exists(vote_backup_file_path.clone()).await {
        Ok(exists) => {
            if exists {
                info!("Vote backup file already exists for today, skipping backup");
                record_vote_backup_duration(MetricResult::Ok, backup_started_at.elapsed());
                return;
            }
        }
        Err(e) => {
            error!("Failed to check if vote backup file exists! Error: {}", e);
            record_pipeline_error(PipelineStage::BackupVotes, PipelineErrorKind::Io);
            record_vote_backup_duration(MetricResult::Error, backup_started_at.elapsed());
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
    record_vote_backup_records(user_votes.len() as u64);

    let backup_votes_file = File::create(&vote_backup_file_path).await;

    if let Err(err) = backup_votes_file {
        error!(
            "Failed to create vote backup file at {}! Error: {}",
            vote_backup_file_path, err
        );
        record_pipeline_error(PipelineStage::BackupVotes, PipelineErrorKind::Io);
        record_vote_backup_duration(MetricResult::Error, backup_started_at.elapsed());
        return;
    }

    let serialized_votes = match serde_json::to_vec_pretty(&user_votes) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize user votes! Error: {}", e);
            record_pipeline_error(PipelineStage::BackupVotes, PipelineErrorKind::Serialize);
            record_vote_backup_duration(MetricResult::Error, backup_started_at.elapsed());
            return;
        }
    };

    let write_return = backup_votes_file
        .unwrap()
        .write_all(&serialized_votes)
        .await;

    if let Err(e) = write_return {
        error!("Failed to write vote backup file! Error: {}", e);
        record_pipeline_error(PipelineStage::BackupVotes, PipelineErrorKind::Io);
        record_vote_backup_duration(MetricResult::Error, backup_started_at.elapsed());
        return;
    }

    info!("Vote backup file written to {}", vote_backup_file_path);
    record_vote_backup_duration(MetricResult::Ok, backup_started_at.elapsed());
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
        let thread_span = info_span!("process_thread_reactions", thread = %thread.name);

        async {
            let Some(meta) = thread.thread_metadata else {
                warn!("Thread '{}' has no metadata, skipping", thread.name);
                return;
            };
            if meta.locked {
                trace!("Ignoring locked thread (probably out-of-date)");
                return;
            }

            let messages = discord.get_all_messages(thread.id).await;

            debug!(
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
        .instrument(thread_span)
        .await;
    }

    users_with_reactions
}

#[instrument(skip_all, fields(new_events_count = %new_events.len()
))]
async fn send_new_events(
    discord: &DiscordAPI,
    new_events: BTreeMap<EventsThread, Vec<Event>>,
    config: &Config,
    category: &Category,
) {
    if new_events.is_empty() {
        info!("No new events to send");
        return;
    }

    if config.debug_config.skip_sending {
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
            async {
                let ticket_url = config.venue_ticket_shop_url.get(&event.venue).cloned();
                let send_started_at = Instant::now();
                let message = match discord
                    .send_event(
                        thread.thread_id,
                        event,
                        ticket_url,
                        &config.ticket_shop_icon_url,
                    )
                    .await
                {
                    Ok(msg) => {
                        record_event_sent(category, MetricResult::Ok);
                        record_event_send_duration(category, send_started_at.elapsed());
                        msg
                    }
                    Err(_) => {
                        record_event_sent(category, MetricResult::Error);
                        record_event_send_duration(category, send_started_at.elapsed());
                        record_pipeline_error(PipelineStage::SendEvents, PipelineErrorKind::Api);
                        return;
                    }
                };

                if config.debug_config.skip_feature_reactions {
                    info!("Skipping feature reactions");
                    return;
                }

                add_feature_reactions(
                    discord,
                    &message,
                    &config.voting_emojis,
                    *SAVE_FOR_LATER_EMOJI,
                )
                .await;
            }
            .await;
        }
    }
}

struct ShutdownHook;

impl Drop for ShutdownHook {
    fn drop(&mut self) {
        info!("App finished")
    }
}
