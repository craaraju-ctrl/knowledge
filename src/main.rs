//! # Knowledge — Web Research Agent
//!
//! Crawls, analyzes, and stores trading/market knowledge into the memory system
//! via the Memory HTTP API (agentic-memory running on port 3111).
//!
//! ## Setup
//!
//! 1. Start the memory API server: `cargo run -p agentic-memory`
//! 2. Get a NewsAPI key at https://newsapi.org/register
//! 3. Run: `cargo run -- <SYMBOL1> <SYMBOL2> ...`

use chrono::Utc;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Constants ───────────────────────────────────────────────────────────────

const NEWSAPI_BASE: &str = "https://newsapi.org/v2";
const DEFAULT_PAGE_SIZE: u32 = 5;
const ENV_KEY_NAME: &str = "NEWSAPI_KEY";

// ── Memory API Client ───────────────────────────────────────────────────────

/// Lightweight client for the Memory HTTP API.
struct MemoryApiClient {
    client: Client,
    base_url: String,
}

impl MemoryApiClient {
    fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Insert a record into memory. Returns the record ID.
    async fn insert_record(&self, record: &MemoryRecord) -> Result<String, String> {
        let url = format!("{}/records", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(record)
            .send()
            .await
            .map_err(|e| format!("Memory API request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Memory API returned {}: {}", status, body));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        json["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No id in response".to_string())
    }

    /// Get memory statistics.
    async fn get_stats(&self) -> Result<MemoryStatsResponse, String> {
        let url = format!("{}/stats", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Memory API stats failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Memory API returned {}: {}", status, body));
        }

        resp.json()
            .await
            .map_err(|e| format!("Failed to parse stats: {}", e))
    }
}

// ── Memory Record Types (mirror of agentic-memory::MemoryRecord) ────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryRecord {
    id: String,
    content: String,
    content_type: String,
    #[serde(default)]
    metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f64>>,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct MemoryStatsResponse {
    total_records: u64,
    #[allow(dead_code)]
    total_with_embeddings: u64,
    content_types: HashMap<String, u64>,
}

// ── NewsAPI Response Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct NewsApiResponse {
    status: String,
    articles: Vec<Article>,
}

#[derive(Debug, Clone, Deserialize)]
struct Article {
    title: Option<String>,
    description: Option<String>,
    url: Option<String>,
    source: Option<Source>,
}

#[derive(Debug, Clone, Deserialize)]
struct Source {
    name: Option<String>,
}

// ── Knowledge Entry ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct KnowledgeEntry {
    topic: String,
    source_label: String,
    title: String,
    summary: String,
    full_text: String,
    article_url: String,
    key_insights: Vec<String>,
    timestamp: String,
    signals: Vec<String>,
}

impl KnowledgeEntry {
    fn from_article(topic: &str, article: &Article, full_text: String) -> Self {
        let title = article.title.as_deref().unwrap_or("Untitled").to_string();
        let summary = article.description.as_deref().unwrap_or("").to_string();
        let article_url = article.url.as_deref().unwrap_or("").to_string();
        let source_label = article
            .source
            .as_ref()
            .and_then(|s| s.name.as_deref())
            .unwrap_or("unknown")
            .to_string();
        let signals = extract_financial_signals(&title, &summary, &full_text);

        Self {
            topic: topic.to_string(),
            source_label,
            title,
            summary,
            full_text,
            article_url,
            key_insights: signals.clone(),
            timestamp: Utc::now().to_rfc3339(),
            signals,
        }
    }

    /// Convert to a MemoryRecord for the HTTP API.
    fn to_memory_record(&self) -> MemoryRecord {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), self.source_label.clone());
        metadata.insert("topic".to_string(), self.topic.clone());
        metadata.insert("article_url".to_string(), self.article_url.clone());
        metadata.insert("signals".to_string(), self.signals.join(","));

        MemoryRecord {
            id: format!("knowledge_{}", Utc::now().timestamp()),
            content: serde_json::to_string(self).unwrap_or_default(),
            content_type: "knowledge".to_string(),
            metadata,
            embedding: None,
            timestamp: self.timestamp.clone(),
        }
    }
}

// ── Financial Signal Extraction ─────────────────────────────────────────────

fn extract_financial_signals(title: &str, desc: &str, body: &str) -> Vec<String> {
    let text = format!("{} {} {}", title, desc, body).to_lowercase();
    let mut signals = Vec::new();

    let signal_map: [(&str, &str, &str); 10] = [
        ("bullish", "upgrade", "positive outlook"),
        ("bearish", "downgrade", "negative outlook"),
        ("earnings", "revenue", "profit"),
        ("volatility", "fluctuation", "swing"),
        ("dividend", "yield", "payout"),
        ("merger", "acquisition", "takeover"),
        ("ipo", "offering", "listing"),
        ("regulation", "sec", "compliance"),
        ("innovation", "partnership", "collaboration"),
        ("risk", "uncertainty", "caution"),
    ];

    for (k1, k2, k3) in &signal_map {
        if text.contains(k1) || text.contains(k2) || text.contains(k3) {
            signals.push(k1.to_string());
        }
    }

    signals.sort();
    signals.dedup();
    signals
}

// ── API Key Resolution ──────────────────────────────────────────────────────

fn resolve_api_key() -> Result<String, String> {
    if let Ok(key) = std::env::var(ENV_KEY_NAME) {
        if !key.is_empty() && key != "your_key_here" {
            return Ok(key);
        }
    }

    let _ = dotenvy::dotenv();
    if let Ok(key) = std::env::var(ENV_KEY_NAME) {
        if !key.is_empty() && key != "your_key_here" {
            return Ok(key);
        }
    }

    Err(format!(
        "{} not set. Get a free key at https://newsapi.org/register, \
         then run: export {}='your_key' (or add it to a .env file)",
        ENV_KEY_NAME, ENV_KEY_NAME
    ))
}

// ── NewsAPI Client ──────────────────────────────────────────────────────────

async fn fetch_market_news(
    client: &Client,
    api_key: &str,
    symbol: &str,
) -> Result<Vec<Article>, String> {
    let url = format!(
        "{}/everything?q={}+stock+market&language=en&sortBy=publishedAt&pageSize={}",
        NEWSAPI_BASE, symbol, DEFAULT_PAGE_SIZE
    );

    let resp = client
        .get(&url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("NewsAPI request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("NewsAPI returned {}: {}", status, body));
    }

    let api_resp: NewsApiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse NewsAPI response: {}", e))?;

    if api_resp.status != "ok" {
        return Err(format!("NewsAPI status not ok: {}", api_resp.status));
    }

    Ok(api_resp.articles)
}

// ── Web Scraping ────────────────────────────────────────────────────────────

async fn scrape_article(client: &Client, url: &str) -> Result<String, String> {
    if url.is_empty() {
        return Ok(String::new());
    }

    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL '{}': {}", url, e))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Ok(String::new());
    }

    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; KnowledgeAgent/1.0)")
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
        .map_err(|_| format!("Failed to fetch {}", url))?;

    if !resp.status().is_success() {
        eprintln!("     \u{26a0}\u{fe0f}  Scrape returned {} \u{2014} skipping", resp.status());
        return Ok(String::new());
    }

    let html = resp.text().await.map_err(|_| "Failed to read body".to_string())?;
    let document = scraper::Html::parse_document(&html);

    let content_selectors = [
        "article", "[role='main']", "main", ".article-content",
        ".post-content", ".entry-content", ".story-body",
        "#article-body", ".content-body", "body",
    ];

    let mut text_parts: Vec<String> = Vec::new();

    for selector_str in &content_selectors {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            for element in document.select(&selector) {
                if let Ok(p_selector) = scraper::Selector::parse("p, h1, h2, h3, h4, li") {
                    for p in element.select(&p_selector) {
                        let t: String = p.text().collect::<Vec<_>>().join(" ");
                        let t = t.trim().to_string();
                        if !t.is_empty() && t.len() > 20 {
                            text_parts.push(t);
                        }
                    }
                }
                if text_parts.len() >= 8 {
                    break;
                }
            }
        }
        if !text_parts.is_empty() {
            break;
        }
    }

    if text_parts.is_empty() {
        if let Ok(body_sel) = scraper::Selector::parse("body") {
            if let Some(body) = document.select(&body_sel).next() {
                text_parts.push(body.text().collect::<Vec<_>>().join(" "));
            }
        }
    }

    let combined = text_parts.join("\n\n");
    Ok(if combined.len() > 5000 {
        format!("{}\u{2026} [truncated]", &combined[..5000])
    } else {
        combined
    })
}

// ── Display ─────────────────────────────────────────────────────────────────

fn display_article(i: usize, entry: &KnowledgeEntry) {
    println!("     \u{250c}\u{2500} Article {} \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", i + 1);
    println!("     \u{2502} \u{1f4f0} {}", entry.title);
    println!("     \u{2502} \u{1f3f7}\u{fe0f}  {}", entry.source_label);
    if !entry.summary.is_empty() {
        println!("     \u{2502} \u{1f4dd} {}", &entry.summary.chars().take(120).collect::<String>());
    }
    if !entry.article_url.is_empty() {
        println!("     \u{2502} \u{1f517} {}", entry.article_url);
    }
    if !entry.signals.is_empty() {
        println!("     \u{2502} \u{1f4ca} Signals: {}", entry.signals.join(", "));
    }
    println!("     \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
}

// ── Research Orchestrator ───────────────────────────────────────────────────

async fn research_symbol(
    client: &Client,
    api_key: &str,
    memory: &MemoryApiClient,
    symbol: &str,
) {
    println!("\u{2500}\u{2500}\u{2500} {} \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", symbol);

    let articles = match fetch_market_news(client, api_key, symbol).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("  \u{26a0}\u{fe0f}  NewsAPI error for {}: {}", symbol, e);
            return;
        }
    };

    if articles.is_empty() {
        println!("  \u{1f4ed} No articles found for {}", symbol);
        return;
    }

    println!("  \u{1f4f0} {} article(s) found. Scraping\u{2026}", articles.len());

    let scrape_tasks: Vec<_> = articles
        .iter()
        .map(|article| {
            let url = article.url.clone().unwrap_or_default();
            async move {
                let text = scrape_article(client, &url).await.unwrap_or_default();
                (url, text)
            }
        })
        .collect();

    let scraped_texts: HashMap<String, String> = futures::future::join_all(scrape_tasks)
        .await
        .into_iter()
        .collect();

    for (i, article) in articles.iter().enumerate() {
        let full_text = article
            .url
            .as_ref()
            .and_then(|u| scraped_texts.get(u))
            .cloned()
            .unwrap_or_default();

        let entry = KnowledgeEntry::from_article(symbol, article, full_text);
        display_article(i, &entry);

        // Save to memory via HTTP API
        let record = entry.to_memory_record();
        match memory.insert_record(&record).await {
            Ok(id) => println!("  \u{2705} Saved to memory (id: {})", id),
            Err(e) => eprintln!("  \u{26a0}\u{fe0f}  {}", e),
        }
    }

    println!();
}

// ── Entry Point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("\u{1f4da} Knowledge \u{2014} Web Research Agent");
    println!("\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");
    println!();

    // \u{2500}\u{2500} API Key \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    let api_key = match resolve_api_key() {
        Ok(k) => {
            println!("\u{1f511} NewsAPI key found");
            k
        }
        Err(e) => {
            eprintln!("\u{274c} {}", e);
            eprintln!();
            eprintln!("\u{1f4a1} Get a free API key at https://newsapi.org/register");
            eprintln!("   Then either:");
            eprintln!("   \u{2022} export NEWSAPI_KEY='your_key_here'");
            eprintln!("   \u{2022} Or create a .env file with: NEWSAPI_KEY=your_key_here");
            std::process::exit(1);
        }
    };

    // \u{2500}\u{2500} Memory API Client \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    let memory_url = std::env::var("MEMORY_API_URL").unwrap_or_else(|_| "http://localhost:3111".to_string());
    let memory = MemoryApiClient::new(&memory_url);
    println!("\u{2705} Memory API client configured: {}", memory_url);

    // Quick health check
    match memory.get_stats().await {
        Ok(stats) => {
            println!("   \u{2139}\u{fe0f}  Memory reports {} existing records", stats.total_records);
        }
        Err(e) => {
            eprintln!("\u{26a0}\u{fe0f}  Could not reach memory API: {}", e);
            eprintln!("   Make sure agentic-memory is running on {}", memory_url);
        }
    }
    println!();

    // \u{2500}\u{2500} HTTP Client (reused across all requests) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("KnowledgeAgent/1.0 (market research bot)")
        .build()
        .expect("Failed to build HTTP client");

    // \u{2500}\u{2500} Symbols to Research \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    let symbols: Vec<String> = std::env::args().skip(1).collect();
    let symbols = if symbols.is_empty() {
        vec!["AAPL".to_string(), "SPY".to_string(), "BTC".to_string(), "TSLA".to_string()]
    } else {
        symbols
    };

    println!("\u{1f50d} Researching: {}\n", symbols.join(", "));

    // \u{2500}\u{2500} Parallel Research \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    let tasks: Vec<_> = symbols
        .iter()
        .map(|sym| research_symbol(&client, &api_key, &memory, sym))
        .collect();

    futures::future::join_all(tasks).await;

    // \u{2500}\u{2500} Summary \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    println!("\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");
    println!();

    match memory.get_stats().await {
        Ok(stats) => {
            println!("\u{1f4ca} Total knowledge entries in memory: {}", stats.total_records);
            let sources: Vec<&String> = stats.content_types.keys().collect();
            if !sources.is_empty() {
                println!("   Content types: {}", sources.iter().join(", "));
            }
        }
        Err(e) => eprintln!("\u{26a0}\u{fe0f}  Could not load memory stats: {}", e),
    }

    println!();
    println!("\u{2705} Knowledge research complete.");
    println!();
    println!("\u{1f4a1} Usage:");
    println!("   knowledge                    \u{2014} research default symbols (AAPL, SPY, BTC, TSLA)");
    println!("   knowledge AAPL GOOGL MSFT    \u{2014} research specific symbols");
    println!("   export NEWSAPI_KEY=...       \u{2014} set your NewsAPI key");
    println!("   export MEMORY_API_URL=...    \u{2014} set memory API URL (default: http://localhost:3111)");
}
