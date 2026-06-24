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
4. Rewrites a sent review's "Comentários" field if the user edits/replaces their reply in DM
5. Backs up vote data to `vote_backups/YYYY_MM_DD.json`

A second, standalone binary (`scripts/backfill_reviews.rs`) lets you inject historical reviews that never went through the live reaction → DM flow (e.g. ratings given before the bot tracked them). See "Backfilling reviews" below.

### Module map

| Module | Purpose |
|---|---|
| `src/agenda_cultural/api.rs` | `AgendaCulturalAPI`: fetch upcoming events from agendalx.pt's REST API (`get_events_by_month`); scrape full HTML description for a known event (`get_full_description`); scrape title/venue/dates/image/description for an arbitrary event URL not in the API anymore (`scrape_event`, used by the backfill script) |
| `src/agenda_cultural/dto.rs` | `EventResponse`: raw REST API shape + its `Deserialize` impls; `to_model` converts it to the domain `Event` |
| `src/agenda_cultural/model.rs` | Domain types: `Event`, `EventDetails`, `Schedule`, `Category` (Teatro/Artes) |
| `src/discord/api.rs` | `DiscordAPI` wrapper around serenity: send event embeds (`send_event`/`build_event_embed`), manage month threads, "save for later" pin reactions, vote-reaction → DM review (`send_privately_users_review`), DM-reply review rewriting (`rewrite_reviews_from_dm_replies`), and historical review backfill (`send_backfill_review`) |
| `src/discord/backup.rs` | Reads bot DMs to reconstruct `VoteRecord` structs and write backup JSON |
| `src/config/` | Load all config from env vars at startup (`Config`, `EmojiConfig`, env parsing helpers) |
| `src/api.rs` | Orchestration: filter unsent events, map events to month-threads, add reaction emojis to new posts |
| `src/main.rs` | Entry point: runs Teatro + Artes pipelines, then triggers vote backup |
| `src/metrics.rs` | OTel metric recorders (events sent/fetched, DM reviews sent, pipeline errors, etc.) |
| `src/tracing.rs` | Sets up stdout + optional Grafana Loki + OTLP tracing/metrics |
| `scripts/backfill_reviews.rs` | Standalone bin: reads a JSON array of `{url, rating, comment}` and sends each as a DM review via `send_backfill_review` |

### Key data flow

`AgendaCulturalAPI::get_events_by_month` → `filter_new_events_by_thread` (dedupes against already-sent Discord embeds) → `send_new_events` (creates/reuses month threads, posts embeds via `build_event_embed`, adds reaction emojis)

Reaction processing (per run, per thread message): `get_user_votes` (reads the 5 star/heart reaction emojis) → `send_privately_users_review` (DMs each voter an embed with "Voto"/"Comentários" fields, skipping if already sent) → if the user later replies in that DM, `rewrite_reviews_from_dm_replies` edits the original review embed's "Comentários" field in place.

### Discord thread structure

Each channel (Teatro / Artes) has public threads named `"<Month PT> <Year>"` (e.g. `"Maio 2026"`). Each event is a single embed message inside the relevant month thread.

### Backfilling reviews

`scripts/backfill_reviews.rs` (bin name `backfill_reviews`) is for reviews that predate the bot's tracking — there's no original event message to copy an embed from, so it scrapes the event page itself (`AgendaCulturalAPI::scrape_event`) to rebuild title/venue/dates/image/description, then sends the same "Voto"/"Comentários" DM format as the live flow (`DiscordAPI::send_backfill_review`).

- Input: JSON array at `scripts/reviews_data.json` (gitignored — contains personal review data) or a path passed as the first CLI arg, shaped `[{"url": "...", "rating": 1-5, "comment": "..."}]`
- Requires `BACKFILL_USER_ID` (Discord user ID to DM) in addition to the standard env vars below
- Skips events it already sent to that user (same dedup check as the live DM flow) and events whose page fails to scrape
- Built and uploaded as a per-architecture release artifact alongside the main binary (see `.github/workflows/release.yaml`)

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
| `OTLP_ENDPOINT` | `http://host:4317` | Optional; gRPC OTLP endpoint (Tempo) |
| `GATHER_NEW_EVENTS` | bool | Default `true`; set `false` to only process reactions |
| `BACKFILL_USER_ID` | integer | `backfill_reviews` only: Discord user ID to send backfilled reviews to |

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
