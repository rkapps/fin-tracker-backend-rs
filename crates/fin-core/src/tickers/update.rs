use agentic_core::client::embeddings::EmbeddingClient;
use anyhow::Result;
use chrono::{Months, Utc};
use fin_domain::{
    tickers::{
        AssetType, ModelAlgorithm, TICKER_PERFORMANCE_PERIODS, Ticker, TickerAlpha, TickerControl,
        TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment,
    },
    utils::data_utils::{
        assets_cap_label, calculate_performance, get_period_close, get_period_start,
    },
};
use fin_providers::ProviderService;
use fin_storage::service::StorageService;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{RwLock, Semaphore},
    time::sleep,
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    ml::prediction::run_predictions,
    tickers::{
        BASE_CURRENCY,
        indicators::IndicatorCalculator,
        signals::SignalsCalculator,
        sync::{should_sync_embeddings, should_sync_history, should_sync_indicators, should_sync_sentiments},
    },
};

pub async fn update_all_tickers(
    storage_service: Arc<dyn StorageService>,
    provider_service: ProviderService,
    embedding_client: Arc<dyn EmbeddingClient>,    
    all_controls: Vec<TickerControl>,
    all_tickers: Vec<Ticker>,
) -> Result<()> {
    let mut control_map: HashMap<String, TickerControl> = all_controls
        .into_iter()
        .map(|c| (c.symbol.clone(), c))
        .collect();

    let total = all_tickers.len();
    let semaphore = Arc::new(Semaphore::new(3));
    let delay = Duration::from_millis(1000);

    info!("Processing {} tickers with 3 concurrent workers", total);

    let tasks: Vec<_> = all_tickers
        .into_iter()
        .enumerate()
        .filter_map(|(i, ticker)| {
            let tc = match control_map.remove(&ticker.symbol) {
                Some(tc) => tc,
                None => {
                    warn!("No control record for {}, skipping", ticker.symbol);
                    return None;
                }
            };

            let sem = semaphore.clone();
            let storage_service = storage_service.clone();
            let provider_service = provider_service.clone();
            let embedding_client = embedding_client.clone();

            Some(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let mut ticker = ticker;
                let mut tc = tc;

                if i % 20 == 0 {
                    info!("Updating Ticker: {} {}/{}", ticker.symbol, i + 1, total);
                }

                let result =
                    update_ticker(storage_service, provider_service, embedding_client, &mut tc, &mut ticker).await;

                sleep(delay).await;

                result.map(|_| (ticker, tc))
            }))
        })
        .collect();

    // collect results
    let results = futures::future::join_all(tasks).await;

    let mut success = 0;
    let mut failed = 0;
    let mut updated_tickers = Vec::new();
    let mut updated_controls = Vec::new();

    for result in results {
        match result {
            Ok(Ok((ticker, tc))) => {
                success += 1;
                updated_tickers.push(ticker);
                updated_controls.push(tc);
            }
            Ok(Err(e)) => {
                error!("Ticker update failed: {}", e);
                failed += 1;
            }
            Err(e) => {
                error!("Task panicked: {}", e);
                failed += 1;
            }
        }
    }

    // bulk write at the end
    if !updated_tickers.is_empty() {
        storage_service.save_tickers(updated_tickers).await?;
        storage_service
            .save_ticker_controls(updated_controls)
            .await?;
    }

    info!("Completed: {} successful, {} failed", success, failed);

    Ok(())
}

pub async fn update_ticker(
    storage_service: Arc<dyn StorageService>,
    provider_service: ProviderService,
    embedding_client: Arc<dyn EmbeddingClient>,
    tc: &mut TickerControl,
    ticker: &mut Ticker,
) -> Result<()> {
    let update = true;

    match update_ticker_details(provider_service.clone(), ticker).await {
        Ok(c) => c,
        Err(e) => {
            let emsg = format!("Ticker update failed for {}: {}", ticker.symbol, e);
            // error!(emsg);
            return Err(anyhow::anyhow!(emsg));
        }
    };

    //Get the history
    let mut histories = Vec::new();

    // update history
    if should_sync_history(tc) {
        match update_ticker_history(provider_service.clone(), tc, ticker).await {
            Ok((all_histories, new_histories)) => {
                if !new_histories.is_empty() {
                    tc.last_history_sync_at = Some(Utc::now());
                    info!(
                        "Ticker new History for {}: {} ",
                        ticker.symbol,
                        new_histories.len()
                    );
                    if update {
                        storage_service.save_ticker_control(tc.clone()).await?;
                        storage_service
                            .save_ticker_history(&ticker.symbol, new_histories)
                            .await?;
                    }
                }
                histories = all_histories
            }
            Err(e) => error!("History update failed for {}: {}", ticker.symbol, e),
        }
    }

    // update sentiments
    if should_sync_sentiments(tc) {
        match update_ticker_sentiments(provider_service, tc, ticker).await {
            Ok(new_sentiments) => {
                if !new_sentiments.is_empty() {
                    debug!(
                        "Ticker {} New Sentiments: {}",
                        ticker.symbol,
                        new_sentiments.len()
                    );
                    tc.last_sentiment_sync_at = Some(Utc::now());
                    if update {
                        storage_service.save_ticker_control(tc.clone()).await?;
                        storage_service
                            .save_ticker_sentiments(&ticker.symbol, new_sentiments)
                            .await?;
                    }
                }
            }
            Err(e) => error!("Sentiments update failed for {}: {}", ticker.symbol, e),
        }
    }

    if should_sync_embeddings(tc) {
        match update_ticker_sentiment_embeddings(
            storage_service.clone(),
            embedding_client,
            tc,
            ticker,
        )
        .await
        {
            Ok(new_embeddings) => {
                if !new_embeddings.is_empty() {
                    debug!(
                        "Ticker {} New Embeddings: {}",
                        ticker.symbol,
                        new_embeddings.len()
                    );
                    tc.last_embedding_sync_at = Some(Utc::now());
                    if update {
                        // storage_service.save_ticker_control(tc.clone()).await?;
                        storage_service
                            .save_ticker_embeddings(&ticker.symbol, new_embeddings)
                            .await?;
                    }
                }
            }
            Err(e) => error!("Embeddings update failed for {}: {}", ticker.symbol, e),
        }
    }

    // update technical indicators
    // tc.last_indicator_sync_at = None;
    if should_sync_indicators(tc) {
        match update_stock_indicators(tc, ticker, &histories).await {
            Ok(new_indicators) => {
                if !new_indicators.is_empty() {
                    debug!(
                        "Ticker {} New Indicators: {}",
                        ticker.symbol,
                        new_indicators.len()
                    );
                    tc.last_indicator_sync_at = Some(Utc::now());
                    if update {
                        storage_service.save_ticker_control(tc.clone()).await?;
                        storage_service
                            .save_ticker_indicators(&ticker.symbol, new_indicators)
                            .await?;
                    }
                }
            }
            Err(e) => error!("Indicators update failed for {}: {}", ticker.symbol, e),
        }
    }

    // sort by descending for updating price history
    histories.sort_by(|a, b| b.date.cmp(&a.date));

    update_ticker_price_history(tc, ticker, &histories).await?;
    //now calculate the performance
    update_ticker_performance(tc, ticker, &histories).await?;

    //update signals
    update_ticker_signals(storage_service.clone(), ticker).await?;

    tc.last_sync_at = Some(Utc::now());

    Ok(())
}

pub async fn update_ticker_realtime(
    provider_service: ProviderService,
    ticker: &mut Ticker,
) -> Result<()> {
    match ticker.asset_type {
        AssetType::Stock | AssetType::Etf => {
            let raw = provider_service
                .get_stock_etf_realtime(&ticker.symbol)
                .await?;

            ticker.update_stock_etf_price_realtime(raw)?;
            debug!(
                "pr date: {:?} pr_prev: {:?} pr_last {:?} pr_diff: {:?}",
                ticker.pr_date, ticker.pr_prev, ticker.pr_last, ticker.pr_diff_amt
            );
        }
        AssetType::Crypto => {}
    }

    Ok(())
}
pub(crate) async fn update_ticker_details(
    provider_service: ProviderService,
    ticker: &mut Ticker,
) -> Result<()> {
    match ticker.asset_type {
        AssetType::Stock => {
            let raw = provider_service.get_stock(&ticker.symbol).await?;
            ticker.update_from_alpha(raw);
        }
        AssetType::Etf => {
            let raw = provider_service.get_etf(&ticker.symbol).await?;
            ticker.update_etf_from_alpha(raw);
        }
        AssetType::Crypto => {
            let raw = provider_service
                .get_crypto(vec![ticker.symbol.clone()])
                .await?;
            ticker.update_crypto_from_cmc(raw);
        }
    }

    Ok(())
}

pub(crate) async fn update_ticker_history(
    provider_service: ProviderService,
    tc: &mut TickerControl,
    ticker: &mut Ticker,
) -> Result<(Vec<TickerHistory>, Vec<TickerHistory>)> {
    let Some(hist_start_date) = Utc::now().checked_sub_months(Months::new(60)) else {
        return Err(anyhow::anyhow!("Error calcuating start date"));
    };

    let histories = match ticker.asset_type {
        AssetType::Stock => {
            let thist = provider_service
                .get_stock_history(&ticker.symbol, &hist_start_date)
                .await?;

            TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?
        }
        AssetType::Etf => {
            let thist = provider_service
                .get_stock_history(&ticker.symbol, &hist_start_date)
                .await?;

            TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?
        }
        AssetType::Crypto => {
            let symbol = format!("{}{}", ticker.symbol, BASE_CURRENCY);
            let mut thist = provider_service
                .get_crypto_history(&symbol, &hist_start_date, "1day")
                .await
                .inspect_err(|e| {
                    warn!("Crypto Ticker history for '{}' error: {}", ticker.symbol, e)
                })?;

            // update adj_close
            for hist in &mut thist {
                hist.adj_close = hist.close;
            }
            TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?
        }
    };

    let mut new_histories = Vec::new();
    if !histories.is_empty() {
        new_histories = match tc.last_history_sync_at {
            Some(last_sync) => histories
                .iter()
                .filter(|h| h.date > last_sync)
                .cloned()
                .collect(),
            None => {
                // First sync - insert all
                histories.clone()
            }
        };
    }
    debug!(
        "Ticker {} New History updates: {}",
        ticker.symbol,
        new_histories.len()
    );

    Ok((histories, new_histories))
}

pub(crate) async fn update_ticker_price_history(
    _tc: &mut TickerControl,
    ticker: &mut Ticker,
    histories: &[TickerHistory],
) -> Result<()> {
    if !histories.is_empty() {
        let last_history = histories[0].clone();
        let mut prev_history = None;
        if histories.len() > 1 {
            prev_history = Some(histories[1].clone());
        }
        // update the price
        trace!(
            "Last History: {:?} Prev history: {:?}",
            last_history.date, prev_history
        );

        ticker.update_price_from_history(last_history, prev_history)?;
    }

    Ok(())
}

pub(crate) async fn update_ticker_performance(
    _tc: &mut TickerControl,
    ticker: &mut Ticker,
    histories: &[TickerHistory],
) -> Result<()> {
    ticker.performance.clear();
    ticker.performance_search.clear();

    for period in TICKER_PERFORMANCE_PERIODS {
        if let Some(start_date) = get_period_start(period)
            && let Some(period_close) = get_period_close(histories, start_date)
        {
            let mut period_map = HashMap::new();
            let mut period_map_search = HashMap::new();
            period_map.insert("price".to_string(), period_close);
            period_map.insert(
                "perc".to_string(),
                calculate_performance(ticker.pr_close, period_close),
            );

            period_map_search.insert("price".to_string(), period_close.to_f64().unwrap_or(0.0));
            period_map_search.insert(
                "perc".to_string(),
                calculate_performance(ticker.pr_close, period_close)
                    .to_f64()
                    .unwrap_or(0.0),
            );

            ticker.performance.insert(period.to_string(), period_map);
            ticker
                .performance_search
                .insert(period.to_string(), period_map_search);
        }
    }
    Ok(())
}

pub(crate) async fn update_ticker_sentiments(
    provider_service: ProviderService,
    tc: &mut TickerControl,
    ticker: &mut Ticker,
) -> Result<Vec<TickerSentiment>> {
    let mut new_sentiments = Vec::new();

    let Some(date_from) = Utc::now().checked_sub_months(Months::new(6)) else {
        return Err(anyhow::anyhow!("Error with DateTime"));
    };
    let feeds = provider_service
        .get_ticker_sentiment(&ticker.symbol, &date_from)
        .await?;
    let feeds_len = feeds.len();
    let sentiments = TickerSentiment::new_from_alpha_batch(&ticker.symbol, feeds);
    debug!(
        "Ticker {} Feeds: {} Sentiments: {}",
        ticker.symbol,
        feeds_len,
        sentiments.len()
    );

    if !sentiments.is_empty() {
        new_sentiments = match tc.last_sentiment_sync_at {
            Some(last_sync) => sentiments
                .into_iter()
                .filter(|h| h.date > last_sync)
                .collect(),
            None => {
                // First sync - insert all
                sentiments
            }
        };
    }
    Ok(new_sentiments)
}

pub(crate) async fn update_ticker_sentiment_embeddings(
    storage_service: Arc<dyn StorageService>,
    embedding_client: Arc<dyn EmbeddingClient>,
    tc: &mut TickerControl,
    ticker: &mut Ticker,
) -> Result<Vec<TickerEmbedding>> {
    let mut new_embeddings = Vec::new();
    let cmp_score = dec!(0.8);

    let all_sentiments = storage_service
        .get_ticker_sentiments_with_score(&ticker.symbol, &cmp_score)
        .await?;

    if all_sentiments.is_empty() {
        return Ok(new_embeddings);
    }

    let sentiments: Vec<_> = all_sentiments
        .into_iter()
        .filter(|s| s.score.abs() > 0.4 && s.date > (Utc::now() - chrono::Duration::days(30)))
        .take(50)
        .collect();

    debug!(
        "Ticker {} Sentiments with score: {} - {}",
        ticker.symbol,
        cmp_score,
        sentiments.len()
    );

    // Generate embeddings
    // Collect the owned Strings so they stay alive
    let mut embedding_texts: Vec<String> = Vec::new();
    let mut sentimentm = HashMap::new();
    for (index, sentiment) in sentiments.into_iter().enumerate() {
        let embedding_text = sentiment.embedding_text();
        embedding_texts.push(embedding_text);
        sentimentm.insert(index, sentiment);
    }

    // Create the references that point to the owned Strings
    let embedding_refs: Vec<&str> = embedding_texts.iter().map(|s| s.as_str()).collect();
    debug!(
        "Ticker {} embeddings: {}",
        ticker.symbol,
        embedding_refs.len()
    );

    let result = match embedding_client.embed_text_batch(&embedding_refs).await {
        Ok(c) => c,
        Err(e) => {
            return Err(anyhow::anyhow!("Embedding error: {}", e));
        }
    };

    let mut embeddings = Vec::new();
    for successful in result.successful {
        let Some(sentiment) = sentimentm.get(&successful.0) else {
            continue;
        };
        let embedding = TickerEmbedding::new(
            &ticker.symbol.to_uppercase(),
            sentiment.date,
            &sentiment.id,
            &sentiment.embedding_text(),
            successful.1.into_vec(),
        );
        embeddings.push(embedding);
    }
    if !embeddings.is_empty() {
        new_embeddings = match tc.last_embedding_sync_at {
            Some(last_sync) => embeddings
                .into_iter()
                .filter(|h| h.date > last_sync)
                .collect(),
            None => {
                // First sync - insert all
                embeddings
            }
        };
    }

    Ok(new_embeddings)
}

pub async fn update_ticker_overview_embedding(
    storage_service: Arc<dyn StorageService>,
    embedding_client: Arc<dyn EmbeddingClient>,
    ticker: &mut Ticker,
) -> Result<()> {
    let assets_cap_label = assets_cap_label(ticker.total_assets);
    // adding the industry twice to increase the weight.
    let overview_text = format!(
        "{} {} {} {} {}",
        ticker.name,
        ticker.sector.as_deref().unwrap_or(""),
        ticker.industry.as_deref().unwrap_or(""),
        assets_cap_label,
        ticker.overview
    );

    match embedding_client.embed_text(&overview_text).await {
        Ok(embedding) => {
            ticker.overview_text = Some(overview_text);
            ticker.overview_embedding = Some(embedding.into_vec());
        }
        Err(e) => error!("Embedding failed for {}: {}", ticker.symbol, e),
    }

    let industry_text = ticker.industry.clone().unwrap_or("".to_string());
    match embedding_client.embed_text(&industry_text).await {
        Ok(embedding) => {
            ticker.industry_text = Some(industry_text);
            ticker.industry_embedding = Some(embedding.into_vec());
        }
        Err(e) => error!("Embedding failed for {}: {}", ticker.symbol, e),
    }

    storage_service.save_ticker(ticker.clone()).await?;

    Ok(())
}

pub(crate) async fn update_stock_indicators(
    tc: &mut TickerControl,
    ticker: &mut Ticker,
    histories: &[TickerHistory],
) -> Result<Vec<TickerIndicator>> {
    let mut new_indicators = Vec::new();

    if !histories.is_empty() {
        let sma_periods = &[20, 50, 100, 200];
        let ema_periods = &[12, 26, 50];
        let rsi_periods = &[10, 14, 26];
        let k_period = 14;
        let d_period = 3;
        let bb_period = 20;
        let bb_std_dev = 2.0;
        let atr_period = 14;
        let volume_ratio_period = 20;

        let indicators = IndicatorCalculator::calculate_all_in_one_pass(
            histories,
            sma_periods.to_vec(),
            ema_periods.to_vec(),
            rsi_periods.to_vec(),
            k_period,
            d_period,
            bb_period,
            bb_std_dev,
            atr_period,
            volume_ratio_period,
        )?;

        new_indicators = match tc.last_indicator_sync_at {
            Some(last_sync) => indicators
                .into_iter()
                .filter(|h| h.date > last_sync)
                .collect(),
            None => {
                // First sync - insert all
                indicators
            }
        };
        debug!(
            "Ticker {} Indicators updates: {}",
            ticker.symbol,
            new_indicators.len()
        );
    }

    Ok(new_indicators)
}

pub(crate) async fn update_ticker_signals(
    storage_service: Arc<dyn StorageService>,
    ticker: &mut Ticker,
) -> Result<()> {
    let window = storage_service
        .get_ticker_indicators_window(&ticker.symbol)
        .await?;

    let price = ticker.pr_last;
    let signalsc = SignalsCalculator {};

    let mut signals = Vec::new();
    signals.extend(signalsc.calculate_sma_stack(&window));
    signals.extend(
        signalsc
            .calculate_sma_50(price, &window)
            .unwrap_or_default(),
    );
    signals.extend(signalsc.calculate_sma_crossover(&window));
    signals.extend(signalsc.calculate_macd_crossover(&window));
    signals.extend(signalsc.calculate_macd_histogram(&window));
    signals.extend(signalsc.calculate_momentum(price, &window));
    signals.extend(signalsc.calculate_trend_exhaustion(price, &window));
    signals.extend(signalsc.calculate_volatility(&window));
    signals.extend(signalsc.calculate_mean_reversion(price, &window));

    signals.extend(
        signalsc
            .calculate_bollinger_bands(price, &window)
            .unwrap_or_default(),
    );
    signals.extend(signalsc.calculate_stochastic(&window));
    signals.extend(signalsc.calculate_rsi(&window).unwrap_or_default());
    signals.extend(
        signalsc
            .calculate_confluence(price, &window)
            .unwrap_or_default(),
    );

    // Oversold using combinatioions
    let oversold_count = [
        signals.contains(&"RSI Oversold".to_string()),
        signals.contains(&"BB Breakout Lower".to_string()),
        signals.contains(&"Stochastic Bearish".to_string()),
        signals.contains(&"Below SMA50".to_string()),
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    if oversold_count >= 3 {
        signals.push("Deeply Oversold".to_string());
    } else if oversold_count >= 2 {
        signals.push("Moderately Oversold".to_string());
    }

    // Beta signals
    if let Some(beta) = ticker.beta {
        if beta < 0.8 {
            signals.push("Low Beta".to_string());
        } else if beta <= 1.2 {
            signals.push("Market Beta".to_string());
        } else if beta <= 1.5 {
            signals.push("High Beta".to_string());
        } else {
            signals.push("Very High Beta".to_string());
        }
    }

    // Analyst consensus
    match ticker.analyst_consensus.as_deref() {
        Some("Strong Buy") => signals.push("Analyst Strong Buy".to_string()),
        Some("Buy") => signals.push("Analyst Buy".to_string()),
        Some("Hold") => signals.push("Analyst Hold".to_string()),
        Some("Sell") => signals.push("Analyst Sell".to_string()),
        Some("Strong Sell") => signals.push("Analyst Strong Sell".to_string()),
        _ => {}
    }

    debug!("Ticker {} signals: {}", ticker.symbol, signals.len());
    ticker.signals = signals;
    Ok(())
}

pub async fn update_ticker_prediction_signals(
    storage_service: Arc<dyn StorageService>,
    ticker: &mut Ticker,
    sas: &[TickerAlpha],
) -> Result<()> {
    let indicators = storage_service
        .get_ticker_indicators_last_n(&ticker.symbol, 2)
        .await?;

    // get() returns Option — convert to Result with ok_or_else
    let indicator = indicators
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("No current indicator for {}", ticker.symbol))?;

    let prev_indicator = indicators.first(); // Option<&TickerIndicator> — None is fine

    // Try ticker model first, fall back to sector
    let ticker_alphas = storage_service
        .get_ticker_alphas_by_key(&ticker.symbol)
        .await
        .unwrap_or_default();

    info!(
        "  Ticker: {} Sector alphas: {} Ticker alphas: {}",
        ticker.symbol,
        sas.len(),
        ticker_alphas.len(),
    );

    // if we do not have data for 4 periods and 2 algos, use the sector alphas
    let alphas = if ticker_alphas.is_empty() || ticker_alphas.len() < 2 {
        sas.to_vec()
    } else {
        ticker_alphas
    };

    let directional_accuracies: Vec<f64> = alphas.iter().map(|f| f.directional_accuracy).collect();
    let min_accuracy = directional_accuracies
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);

    let returns = run_ticker_predictions(indicator, prev_indicator, &alphas, sas.to_vec()).await?;

    let mlp_alphas: HashMap<String, &TickerAlpha> = alphas
        .iter()
        .filter(|a| matches!(a.model_algorithm, ModelAlgorithm::MLP))
        .map(|a| (a.n.to_string(), a))
        .collect();

    debug!("    Returns: {:?}", returns);

    let signalsc = SignalsCalculator {};

    let ml_signals = signalsc.calculate_ml_signals(
        &returns.0,
        Some(&returns.1),
        &returns.2,
        &mlp_alphas,
        min_accuracy,
    );
    ticker.lr_returns = returns.0;
    ticker.rf_returns = returns.1;
    ticker.mlp_returns = returns.2;

    info!("    Ml Signals: {:?}", ml_signals);
    // Remove any existing LR signals
    ticker.signals.retain(|s| {
        !s.starts_with("LR")
            && !s.starts_with("RF")
            && !s.starts_with("ML")
            && !s.starts_with("MLP")
    });
    // Add fresh ones
    ticker.signals.extend(ml_signals);

    Ok(())
}

/// Prefer ticker model, fall back to sector model
pub async fn run_ticker_predictions(
    indicator: &TickerIndicator,
    prev_indicator: Option<&TickerIndicator>,
    ticker_alphas: &Vec<TickerAlpha>,
    sector_alphas: Vec<TickerAlpha>,
) -> Result<(
    HashMap<String, f64>,
    HashMap<String, f64>,
    HashMap<String, f64>,
)> {
    let sas = if !ticker_alphas.is_empty() {
        info!("    Using ticker model for {}", indicator.symbol);
        ticker_alphas
    } else if !sector_alphas.is_empty() {
        info!("  Falling back to sector model for {}", indicator.symbol);
        &sector_alphas
    } else {
        return Err(anyhow::anyhow!(
            "Ticker alphs not found for symbol: {}",
            indicator.symbol
        ));
    };

    let rf_models = Arc::new(RwLock::new(HashMap::new()));
    run_predictions(rf_models, indicator, prev_indicator, sas.to_vec()).await
}
