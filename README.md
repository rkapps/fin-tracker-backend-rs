# fin-tracker

An agentic stock market analysis platform built in Rust. Combines real-time market data, technical indicators, sentiment analysis, and multi-LLM reasoning to deliver institutional-quality stock analysis through a natural language interface.

---

## Overview

fin-tracker is a backend-driven financial analysis system that lets users query the stock market in plain English. Ask it to compare stocks, screen for opportunities, or deep dive into a specific company — it fetches live data, computes technical signals, analyses sentiment, and synthesises everything into a structured analysis with actionable recommendations.

The system uses an agentic completion loop: the LLM decides which tools to call, executes them concurrently, and iterates until it has enough data to generate a response. Users never need to know which ticker symbols to use or which metrics to look at.

---

## Features

### Natural Language Queries

- "Compare Dutch Bros to its peers"
- "Find oversold mid cap software infrastructure stocks"
- "Do a deep dive on Apple"
- "Compare market SPDR ETFs"
- "Find defensive buy-rated stocks"

### Agentic Tool Execution

The LLM autonomously decides which tools to call based on the query, executes all tool calls concurrently within each iteration, and iterates until analysis is complete.

### Stock Analysis Tools

| Tool                  | Description                                                                |
|-----------------------|----------------------------------------------------------------------------|
| `ticker_snapshot`     | Price, fundamentals, performance across 1W/1M/3M/6M/1Y/2Y/5Y/YTD           |
| `ticker_indicator`    | RSI, MACD, SMA, EMA, Bollinger Bands, ATR, Stochastic                      |
| `ticker_sentiment`    | Vector similarity search over recent news articles                         |
| `ticker_peers`        | Industry peers with sector similarity fallback                             |
| `ticker_screening`    | Multi-criteria screening with semantic search and signal filters           |
| `ticker_taxonomy`     | Available sectors and exact industry strings for precise filtering         |
| `price_history`       | OHLCV history with high/low/trend narrative                                |

### Pre-computed Signals

Signals are computed at EOD and stored on each ticker for fast screening queries:

**Trend:** Golden Cross, Death Cross, Above/Below SMA50, Above/Below SMA200  
**Momentum:** MACD Bullish/Bearish Crossover  
**RSI:** Oversold, Overbought  
**Bollinger Bands:** Breakout Upper/Lower, Squeeze  
**Stochastic:** Bullish/Bearish Crossover  
**Analyst:** Strong Buy, Buy, Hold, Sell, Strong Sell  
**Beta:** Low Beta, Market Beta, High Beta, Very High Beta

### Semantic Similarity Search

Stocks are embedded using their name, sector, industry, market cap classification, and business overview. Similarity search finds thematically related stocks without requiring exact ticker knowledge. Combined with industry and signal filters for precise screening.

### Multi-LLM Support

Supports Anthropic Claude, OpenAI GPT, and Google Gemini via a provider-agnostic completion interface. Switch providers without changing application code.

### Response Formats

- **Summary** — compact comparison table with key metrics and 2-3 sentence synopsis
- **Detail** — full breakdown with FUNDAMENTALS, TECHNICALS, PRICE HISTORY, and SENTIMENT sections

---

## Architecture

```
┌─────────────────────────────────────────────┐
│                Angular Frontend              │
└─────────────────────────┬───────────────────┘
                          │ HTTP
┌─────────────────────────▼───────────────────┐
│              Axum API (bins/api)             │
│         /stocks/analyse endpoint            │
└─────────────────────────┬───────────────────┘
                          │
┌─────────────────────────▼───────────────────┐
│           agentic-core-rs (Agent)            │
│                                             │
│  complete_with_tools()                      │
│  ┌─────────────────────────────────────┐   │
│  │ Loop until no tool calls:           │   │
│  │  1. Call LLM                        │   │
│  │  2. Execute all tools concurrently  │   │
│  │  3. Append results to messages      │   │
│  └─────────────────────────────────────┘   │
└──────────┬──────────────────────┬──────────┘
           │                      │
┌──────────▼──────────┐  ┌───────▼──────────┐
│   fin-analysis-rs   │  │   LLM Providers  │
│                     │  │                  │
│  Tool Registry      │  │  Anthropic       │
│  - snapshot         │  │  OpenAI          │
│  - indicator        │  │  Gemini          │
│  - sentiment        │  └──────────────────┘
│  - peers            │
│  - screening        │
│  - taxonomy         │
│  - price_history    │
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│   storage-core-rs   │
│                     │
│  MongoDB            │
│  - Ticker           │
│  - TickerHistory    │
│  - TickerIndicator  │
│  - TickerEmbedding  │
│  - SentimentFeed    │
│  Vector Search      │
└─────────────────────┘
```

---

## Data Pipeline

### EOD Update (runs daily)

1. **Price history** — fetches OHLCV from market data provider, stores split-adjusted close for accurate performance calculations
2. **Fundamentals** — updates P/E, EPS, market cap, analyst consensus, price targets
3. **Indicators** — computes RSI, MACD, SMA 20/50/100/200, EMA 12/26/50, Bollinger Bands, ATR, Stochastic in a single pass. Stored as one document per ticker per day
4. **Signals** — computes trading signals from indicator crossovers and thresholds, stored as `Vec<String>` on the ticker document
5. **Sentiment** — fetches recent news, generates embeddings, stores top articles per ticker

### Overview Embedding (manual)

Generates semantic embeddings for each ticker combining name, sector, industry, market cap classification, and business overview. Used for similarity-based stock discovery.

---

## Data Model

### Ticker

```
symbol, name, sector, industry, asset_type
price: { last, prev, open, high, low, change, 52wk_high, 52wk_low }
fundamentals: { market_cap, yield, eps, pe_ratio, forward_pe, peg, pb, ps, beta, analyst_consensus, analyst_target }
performance: HashMap<period, { price, perc }>   // split-adjusted
signals: Vec<String>                             // pre-computed EOD signals
overview_embedding: Vec<f32>                     // semantic similarity
industry_embedding: Vec<f32>                     // industry similarity
```

### TickerIndicator

Single document per ticker per day with all indicators in a `HashMap<String, Decimal>`:

```
sma_20, sma_50, sma_100, sma_200
ema_12, ema_26, ema_50
rsi_10, rsi_14, rsi_26
macd, macd_signal, macd_histogram
bb_upper, bb_middle, bb_lower
atr, stochastic_k_14, stochastic_d
```

---

## Crate Structure

```
fin-tracker/
├── bins/
│   └── api/                    # Axum HTTP server
│   └── cron/                   # Axum HTTP serve
├── crates/
│   ├── fin-analysis/           # Tool implementations, agent setup
│   ├── fin-domain/             # Domain models (Ticker, TickerIndicator, etc.)
│   └── fin-http/               # Http Client
│   └── fin-providers/          # Providers for market data
│   └── fin-services/           # Application services
│   └── fin-storage/            # Application storage
---

## Configuration

```toml
[llm]
provider = "gemini"             # anthropic | openai | gemini
model = "gemini-3-flash-preview"
temperature = 0.3
max_tokens = 3000

[mongodb]
uri = "mongodb+srv://..."
database = "fin_tracker"

[market_data]
provider = "alphavantage"
api_key = "..."

[embeddings]
provider = "openai"
model = "text-embedding-3-small"
```

---

## Example Queries

**Peer comparison**
> "Compare Coinbase with its peers"

**Theme screening**
> "Find oversold mid cap software infrastructure stocks"

**Signal screening**
> "Find defensive buy-rated stocks"

**ETF comparison**
> "Compare market SPDR ETFs"

**Deep dive**
> "Do a deep dive on Dutch Bros and peers"

**Similarity search**
> "Compare medical devices stocks"

---

## LLM Provider Notes

| Provider    | Quality       | Cost                  | Consistency |
|-------------|---------------|-----------------------|-------------|
| Gemini      | Excellent     | Free tier             | High        |
| Anthropic   | Excellent     | ~$3/day casual use    | High        |
| OpenAI      | Good          | Low                   | Moderate    |

Gemini is the recommended provider for production use given the free tier availability and high output quality.

---

## Technical Notes

- All performance calculations use split-adjusted close prices to handle stock splits correctly
- Indicators stored as a single HashMap document per ticker per day — 95% storage reduction vs per-indicator documents
- Tool calls within each agent iteration execute concurrently via `futures::future::join_all`
- Sentiment search uses cosine similarity over OpenAI embeddings, truncated to 150 characters per article to reduce LLM context size
- Sector/industry taxonomy tool ensures LLM uses exact industry strings for precise MongoDB filtering
