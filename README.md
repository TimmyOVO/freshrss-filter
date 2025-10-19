# freshrss-filter

LLM-powered filter for FreshRSS that periodically reviews unread items, classifies ads/sponsored content, and takes action (mark read or label).

---

# freshrss-filter

基于 LLM 的 FreshRSS 过滤器，定期审查未读项目，分类广告/赞助内容，并执行操作（标记为已读或添加标签）。

## Features

- Periodic processing via cron-style scheduler
- OpenAI-based classification with configurable prompt and threshold
- Dedup review persistence in SQLite
- Actions: mark-as-read (Fever API) or add label (GReader API)
- Dry-run mode to audit without modifying FreshRSS

## 功能特性

- 通过 cron 风格调度器定期处理
- 基于 OpenAI 的分类，支持自定义提示和阈值
- 在 SQLite 中持久化去重审查记录
- 操作：标记为已读（Fever API）或添加标签（GReader API）
- 干运行模式，可在不修改 FreshRSS 的情况下审计

## Requirements

- FreshRSS instance with Fever API enabled
- Optional: GReader API credentials for labeling
- OpenAI API key
- Rust toolchain (cargo) for building

## 系统要求

- 启用了 Fever API 的 FreshRSS 实例
- 可选：用于标签功能的 GReader API 凭据
- OpenAI API 密钥
- 用于构建的 Rust 工具链 (cargo)

## Install & Build

```bash
cargo build --release
```

The binary will be at `target/release/freshrss-filter`.

## 安装与构建

```bash
cargo build --release
```

可执行文件将位于 `target/release/freshrss-filter`。

## Configuration

Copy `config.example.toml` to `config.toml` and adjust:

- `[openai]`
  - `api_key`: your key
  - `model`, `system_prompt`, `threshold`: optional tuning
- `[freshrss]`
  - `base_url`: your FreshRSS URL
  - `fever_api_key`: Fever API key from FreshRSS user settings (generated as: `api_key=$(echo -n "username:freshrss" | md5sum | cut -d' ' -f1)`)
  - `delete_mode`: `mark_read` or `label`
  - `greader_username`/`greader_password`: required for `label` mode
  - `spam_label`: label name, default `Ads`
- `[scheduler]`
  - `cron`: default every 10 minutes (`0 */10 * * * *`)
- `[database]`
  - `path`: sqlite file path
- Top-level `dry_run`: true to avoid write actions

## 配置

复制 `config.example.toml` 为 `config.toml` 并调整：

- `[openai]`
  - `api_key`: 您的 API 密钥
  - `model`, `system_prompt`, `threshold`: 可选调优参数
- `[freshrss]`
  - `base_url`: 您的 FreshRSS URL
  - `fever_api_key`: 来自 FreshRSS 用户设置的 Fever API 密钥（生成方法：`api_key=$(echo -n "用户名:freshrss" | md5sum | cut -d' ' -f1)`）
  - `delete_mode`: 删除模式：`mark_read` 或 `label`
  - `greader_username`/`greader_password`: `label` 模式所需
  - `spam_label`: 标签名称，默认为 `Ads`
- `[scheduler]`
  - `cron`: 默认每 10 分钟运行一次
- `[database]`
  - `path`: SQLite 文件路径
- 顶级 `dry_run`：设为 true 可避免写入操作

## Usage

- Run with scheduler:
```bash
cargo run
```
- One-off run:
```bash
cargo run -- --once
```
- Dry-run mode:
```bash
cargo run -- --dry-run
```
- Specify config path:
```bash
cargo run -- --config /path/to/config.toml
```

## 使用方法

- 带调度器运行：
```bash
cargo run
```
- 单次运行：
```bash
cargo run -- --once
```
- 干运行模式：
```bash
cargo run -- --dry-run
```
- 指定配置文件路径：
```bash
cargo run -- --config /path/to/config.toml
```

## Actions

- `mark_read`: marks classified ads as read via Fever API
- `label`: adds `spam_label` to the item using GReader `/reader/api/0/edit-tag` endpoint, then marks read

## 操作说明

- `mark_read`: 通过 Fever API 将分类的广告标记为已读
- `label`: 使用 GReader `/reader/api/0/edit-tag` 端点为项目添加 `spam_label`，然后标记为已读

## Notes

- Fever API does not hard-delete items; labeling keeps the inbox cleaner while allowing review
- DB table `reviews` prevents re-reviewing the same item by `item_id`
- The LLM response must be valid JSON with fields: `is_ad`, `confidence`, `reason`

## 注意事项

- Fever API 不会硬删除项目；标签功能可在保持收件箱整洁的同时允许审查
- 数据库表 `reviews` 通过 `item_id` 防止重复审查同一项目
- LLM 响应必须是包含以下字段的有效 JSON：`is_ad`、`confidence`、`reason`

## Roadmap

- More robust FreshRSS API integration (e.g., moving to a dedicated category via API)
- Unit tests for classification thresholds and DB behavior

## 路线图

- 更强大的 FreshRSS API 集成（例如通过 API 移动到专用类别）
- 分类阈值和数据库行为的单元测试

