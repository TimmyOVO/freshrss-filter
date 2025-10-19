# freshrss-filter

LLM-powered filter for FreshRSS that periodically reviews unread items, classifies ads/sponsored content, and takes action (mark read or label).

🤖 **AI-Powered Content Curation** - Automatically filter out ads, sponsored content, and low-quality articles from your RSS feeds using advanced LLM analysis.

🔄 **Set It & Forget It** - Runs automatically in the background via cron scheduler, keeping your RSS feeds clean without manual intervention.

⚡ **Smart & Efficient** - Maintains a review history to avoid re-processing the same items, and provides detailed confidence scores for each classification.

🦀 **Lightweight & Fast** - Built with Rust for exceptional performance and minimal resource usage, perfect for running 24/7 on NAS devices or low-power servers without impact on other services.

📊 **Flexible Actions** - Choose between marking spam as read or applying custom labels, with full support for both Fever and GReader APIs.

🔒 **Privacy-First** - Works with your self-hosted FreshRSS instance, keeping your reading habits and content private.

---

# freshrss-filter

基于 LLM 的 FreshRSS 过滤器，定期审查未读项目，分类广告/赞助内容，并执行操作（标记为已读或添加标签）。

🤖 **AI驱动的内容策展** - 使用先进的大语言模型分析技术，自动过滤RSS源中的广告、赞助内容和低质量文章。

🔄 **设置后无需干预** - 通过cron调度器在后台自动运行，持续保持RSS源的清洁，无需手动干预。

⚡ **智能高效** - 维护审查历史记录避免重复处理相同项目，为每次分类提供详细的置信度评分。

🦀 **轻量快速** - 使用Rust构建，具有卓越的性能和极低的资源占用，非常适合在NAS设备或低功耗服务器上24/7运行，不会对其他服务产生影响。

📊 **灵活的操作方式** - 支持标记垃圾内容为已读或应用自定义标签，完全兼容Fever和GReader API。

🔒 **隐私优先** - 与您自托管的FreshRSS实例配合工作，确保阅读习惯和内容隐私安全。

## Why This Project?

RSS feeds are an excellent way to stay informed, but as content creators increasingly rely on sponsored content and advertisements, our RSS readers have become cluttered with items that don't serve our interests. Manually sifting through dozens or hundreds of articles daily to identify and remove promotional content is time-consuming and tedious.

This project automates the content curation process using modern AI technology. By leveraging Large Language Models' advanced understanding of context and intent, we can accurately identify promotional material, sponsored posts, and low-quality content that doesn't align with your reading goals.

### What It Solves

- **Information Overload**: Reduces noise from commercial content, letting you focus on valuable information
- **Time Savings**: Eliminates manual review of each article for promotional content
- **Better Reading Experience**: Maintains a clean, high-quality RSS feed that matches your interests
- **Consistent Filtering**: Applies the same quality standards across all your RSS sources

### Who This Is For

- RSS power users managing dozens or hundreds of feeds
- Professionals who rely on RSS for industry news and insights
- Anyone who values their reading time and wants to minimize exposure to promotional content
- Self-hosting enthusiasts who prefer to keep their data private while enjoying modern AI benefits

## 项目背景

RSS是获取信息的好方法，但随着内容创作者越来越依赖赞助内容和广告，我们的RSS阅读器中充斥着不符合我们兴趣的项目。每天手动筛选几十或数百篇文章来识别和删除推广内容既耗时又乏味。

这个项目使用现代AI技术自动化内容策展过程。通过利用大语言模型对上下文和意图的先进理解，我们可以准确识别与您的阅读目标不符的推广材料、赞助帖子和低质量内容。

### 解决的问题

- **信息过载**：减少商业内容的噪音，让您专注于有价值的信息
- **节省时间**：无需手动审查每篇文章的推广内容
- **更好的阅读体验**：保持符合您兴趣的清洁、高质量的RSS源
- **一致性过滤**：在所有RSS源中应用相同的质量标准

### 适用人群

- 管理数十或数百个RSS源的重度用户
- 依赖RSS获取行业新闻和见解的专业人士
- 珍惜阅读时间并希望尽量减少接触推广内容的人
- 偏好保护数据隐私同时享受现代AI益处的自托管爱好者

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

### Performance & Resources / 性能与资源

Built with Rust's memory safety and zero-cost abstractions, this filter operates with minimal CPU and RAM usage. The efficient design ensures smooth operation even on resource-constrained environments like NAS devices or single-board computers, making it ideal for 24/7 automated processing without affecting system performance.

使用Rust的内存安全和零成本抽象构建，此过滤器以最小的CPU和RAM占用运行。高效的设计确保即使在NAS设备或单板计算机等资源受限的环境中也能流畅运行，使其成为24/7自动化处理的理想选择，不会影响系统性能。

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

