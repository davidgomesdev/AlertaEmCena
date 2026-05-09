# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                     # build
cargo test                      # run all tests
cargo test <test_name>          # run single test
make check                      # cargo fmt + clippy -D warnings
make fmt                        # cargo fmt + clippy --fix
```

Tests use `test-log` and `tokio::test`. Integration tests in `tests/` require live Discord/API access; unit tests in `src/` use local fixtures from `res/tests/`.

## Architecture

One-shot async Rust binary (tokio). On each run it:

1. Scrapes new theater/arts events from the `agendalx.pt` REST API + HTML
2. Posts new events as Discord embeds into per-month threads in two channels (Teatro, Artes)
3. Processes reactions on existing posts: updates "save for later" pins and sends vote DMs
4. Backs up vote data to `vote_backups/YYYY_MM_DD.json`

### Module map

| Module | Purpose |
|---|---|
| `src/agenda_cultural/` | Fetch events from agendalx.pt; parse HTML descriptions; map to `Event` model |
| `src/discord/api.rs` | `DiscordAPI` wrapper around serenity: send embeds, manage threads, reactions, DMs |
| `src/discord/backup.rs` | Reads bot DMs to reconstruct `VoteRecord` structs and write backup JSON |
| `src/config/` | Load all config from env vars at startup |
| `src/api.rs` | Orchestration: filter unsent events, map events to month-threads |
| `src/main.rs` | Entry point: runs Teatro + Artes pipelines, then triggers vote backup |
| `src/tracing.rs` | Sets up stdout + optional Grafana Loki logging |

### Key data flow

`AgendaCulturalAPI::get_events_by_month` → `filter_new_events_by_thread` (dedupes against already-sent Discord embeds) → `send_new_events` (creates/reuses month threads, posts embeds, adds reaction emojis)

### Discord thread structure

Each channel (Teatro / Artes) has public threads named `"<Month PT> <Year>"` (e.g. `"Maio 2026"`). Each event is a single embed message inside the relevant month thread.

## Required Environment Variables

| Variable | Format | Notes |
|---|---|---|
| `DISCORD_TOKEN` | string | Bot token |
| `DISCORD_TEATRO_CHANNEL_ID` | integer | Channel ID |
| `DISCORD_ARTES_CHANNEL_ID` | integer | Channel ID |
| `VOTING_EMOJIS` | `name:ID;name:ID;...` | Exactly 5, worst→best |
| `VENUE_TICKET_SHOP_URLS` | `venue:url;venue:url;...` | Semicolon-separated |
| `TICKET_SHOP_ICON_URL` | URL | Icon shown on ticket link |
| `LOKI_URL` | `https://user:token@host` | Optional; skipped if missing or unreachable |
| `OTLP_ENDPOINT` | `http://host:4317` | Optional; gRPC OTLP endpoint (Grafana Alloy) |
| `GATHER_NEW_EVENTS` | bool | Default `true`; set `false` to only process reactions |

### Debug variables (all optional)

| Variable | Default | Effect |
|---|---|---|
| `DEBUG_CLEAR_CHANNEL` | `false` | Delete all messages in both channels |
| `DEBUG_EXIT_AFTER_CLEARING` | `false` | Exit immediately after clearing |
| `DEBUG_SKIP_SENDING` | `false` | Fetch events but don't post them |
| `DEBUG_SKIP_FEATURE_REACTIONS` | `false` | Skip reaction processing |
| `DEBUG_SKIP_ARTES` | `false` | Only run Teatro pipeline |
| `DEBUG_EVENT_LIMIT` | unset | Limit events fetched per category |

See `run-locally.sh` for a working local configuration example.
