# 📚 Knowledge — Web Research Agent

**Knowledge** is a Rust-based web research agent that fetches market news via NewsAPI, scrapes article content, extracts financial signals, and stores structured knowledge entries into the **agentic-memory** system for long-term retrieval and analysis.

```
  Symbol → NewsAPI → Scrape → Signal Extraction → Memory Storage
```

---

## Features

- **Multi-Symbol Research** — Research multiple tickers in parallel
- **NewsAPI Integration** — Fetches latest market news for any symbol
- **Article Scraping** — Extracts full article content from news sources
- **Financial Signal Detection** — Auto-tags articles with signals (bullish, bearish, earnings, volatility, etc.)
- **Memory Persistence** — Stores structured knowledge entries in agentic-memory
- **Progress Feedback** — Real-time console output with article details and save status

## Quick Start

```bash
# Prerequisites: Start agentic-memory on port 3111
cd Memory && cargo run

# Get a NewsAPI key
# 1. Go to https://newsapi.org/register
# 2. Get a free API key

# Set your API key
export NEWSAPI_KEY='your_key_here'

# Research default symbols (AAPL, SPY, BTC, TSLA)
cargo run

# Research specific symbols
cargo run -- AAPL GOOGL MSFT NVDA
```

## Usage

```bash
# Research default symbols
knowledge

# Research specific symbols
knowledge AAPL GOOGL MSFT

# With custom memory API
export MEMORY_API_URL=http://localhost:3111
knowledge BTC ETH SOL
```

### Environment Variables

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `NEWSAPI_KEY` | — | ✅ | NewsAPI API key (get at [newsapi.org](https://newsapi.org/register)) |
| `MEMORY_API_URL` | `http://localhost:3111` | ❌ | Memory API server URL |

You can also create a `.env` file in the project root:

```bash
NEWSAPI_KEY=your_key_here
MEMORY_API_URL=http://localhost:3111
```

## Output

For each symbol researched, Knowledge displays:

```
───── AAPL ────────────────────────────────────────
  📰 Article 1
  ├ 📰 Apple Reports Record Quarterly Revenue
  ├ 🏷️  Reuters
  ├ 📝 Apple's quarterly revenue surged past analyst expectations...
  ├ 🔗 https://reuters.com/article/...
  └ 📊 Signals: bullish, earnings, innovation
  ✅ Saved to memory (id: knowledge_1712345678)
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Knowledge Agent                     │
│                                                        │
│  ┌──────────┐    ┌──────────┐    ┌───────────────┐   │
│  │ NewsAPI  │───▶│ Scraper  │───▶│ Signal        │   │
│  │ Fetcher  │    │ (HTML)   │    │ Extraction    │   │
│  └──────────┘    └──────────┘    └───────┬───────┘   │
│                                          │           │
│                                          ▼           │
│                                   ┌──────────────┐   │
│                                   │ Memory API   │   │
│                                   │ (Store)      │   │
│                                   └──────────────┘   │
└─────────────────────────────────────────────────────┘
```

### Signal Detection

Knowledge automatically detects financial signals in article content:

| Signal | Keywords |
|--------|----------|
| **bullish** | upgrade, positive outlook, growth |
| **bearish** | downgrade, negative outlook, decline |
| **earnings** | revenue, profit, quarterly |
| **volatility** | fluctuation, swing, unstable |
| **dividend** | yield, payout, dividend |
| **merger** | acquisition, takeover, M&A |
| **ipo** | offering, listing, public |
| **regulation** | sec, compliance, regulatory |
| **innovation** | partnership, collaboration, R&D |
| **risk** | uncertainty, caution, warning |

## Memory Integration

Each research session stores structured knowledge entries in the Memory API:

```json
{
  "id": "knowledge_1712345678",
  "content": "{\"topic\":\"AAPL\",\"title\":\"...\",\"signals\":[\"bullish\",\"earnings\"],...}",
  "content_type": "knowledge",
  "metadata": {
    "source": "Reuters",
    "topic": "AAPL",
    "signals": "bullish,earnings"
  }
}
```

Search stored knowledge:

```bash
# Using Tantra
tantra search "AAPL bullish"

# Direct API call
curl http://localhost:3111/search/smart?q=earnings+AAPL
```

## Dependencies

- **Rust** 1.70+ (edition 2021)
- **agentic-memory** — Running on port 3111 (or configured `MEMORY_API_URL`)
- **NewsAPI key** — Free tier at [newsapi.org](https://newsapi.org/register)

### Rust Crates

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client for NewsAPI and article scraping |
| `scraper` | HTML parsing and content extraction |
| `serde` / `serde_json` | JSON serialization/deserialization |
| `chrono` | Timestamp generation |
| `itertools` | Collection utilities |
| `futures` | Async parallel execution |
| `dotenvy` | `.env` file loading |
| `url` | URL validation |

## Development

```bash
# Build
cargo build

# Run with default symbols
cargo run

# Run with custom symbols
cargo run -- AAPL GOOGL MSFT

# Check for issues
cargo clippy
cargo fmt --check

# Build release
cargo build --release
```

## Related Projects

- [Tredo](https://github.com/craaraju-ctrl/Tredo) — Agentic AI Trading System
- [Memory](https://github.com/craaraju-ctrl/memory) — Agentic Memory System
- [Tantra](https://github.com/craaraju-ctrl/tantra) — Engineering Loop Orchestrator
