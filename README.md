# freshrss-filter

LLM-powered filter for FreshRSS that periodically reviews unread items, classifies ads/sponsored content, and takes action (mark read or label). Includes a simple TUI for status.

## Features
- Periodic processing via cron-style scheduler
- OpenAI-based classification with configurable prompt and threshold
- Dedup review persistence in SQLite
- Actions: mark-as-read (Fever API) or add label (GReader API)
- Dry-run mode to audit without modifying FreshRSS
- Minimal TUI showing latest run status

## Requirements
- FreshRSS instance with Fever API enabled
- Optional: GReader API credentials for labeling
- OpenAI API key
- Rust toolchain (cargo) for building

## Install & Build
```
cargo build --release
```
The binary will be at `target/release/freshrss-filter`.

## Configuration
Copy `config.example.toml` to `config.toml` and adjust:

- `[openai]`
  - `api_key`: your key
  - `model`, `system_prompt`, `threshold`: optional tuning
- `[freshrss]`
  - `base_url`: your FreshRSS URL
  - `fever_api_key`: Fever API key from FreshRSS user settings
  - `delete_mode`: `mark_read` or `label`
  - `greader_username`/`greader_password`: required for `label` mode
  - `spam_label`: label name, default `Ads`
- `[scheduler]`
  - `cron`: default every 10 minutes (`0 */10 * * * *`)
- `[database]`
  - `path`: sqlite file path
- Top-level `dry_run`: true to avoid write actions

Environment overrides use prefix `FRF__` with double underscores to nest:
- `FRF__OPENAI__API_KEY`
- `FRF__FRESHRSS__BASE_URL`
- `FRF__FRESHRSS__FEVER_API_KEY`
- `FRF__FRESHRSS__DELETE_MODE`
- `FRF__FRESHRSS__GREADER_USERNAME`
- `FRF__FRESHRSS__GREADER_PASSWORD`
- `FRF__FRESHRSS__SPAM_LABEL`
- `FRF__SCHEDULER__CRON`
- `FRF__DATABASE__PATH`
- `FRF__DRY_RUN`

## Usage
- Run with scheduler + TUI:
```
cargo run
```
- One-off run (no TUI):
```
cargo run -- --once
```
- Dry-run mode:
```
cargo run -- --dry-run
```
- Specify config path:
```
cargo run -- --config /path/to/config.toml
```

Press `q` to exit the TUI.

## Actions
- `mark_read`: marks classified ads as read via Fever API.
- `label`: adds `spam_label` to the item using GReader `/reader/api/0/edit-tag` endpoint, then marks read.

## Notes
- Fever API does not hard-delete items; labeling keeps the inbox cleaner while allowing review.
- DB table `reviews` prevents re-reviewing the same item by `item_id`.
- The LLM response must be valid JSON with fields: `is_ad`, `confidence`, `reason`.

## Roadmap
- More robust FreshRSS API integration (e.g., moving to a dedicated category via API)
- Expanded TUI: history list and log streaming
- Unit tests for classification thresholds and DB behavior

