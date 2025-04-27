# Alerta Em Cena

Notifies about the latest theater and musical pieces happening.

## Running

### Configuration

Read in [load_config](src/config/env_loader.rs).

Set LOKI_URL to `https://<your user id>:<Your Grafana.com API Token>@<loki instance>.grafana.net`.

### Voting emojis

Add 5 emojis on the bot. (symbolizing worst to best)

The ones in [res/voting-emojis](res/voting-emojis) are from https://emoji.gg/user/teganchan.

Provide their `name:ID` semi-colon separated in the env variables. (e.g. `1_:123;2_:234;...`)
