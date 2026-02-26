use chrono::{Duration, Months, Utc};
use fin_domain::{
    ticker::{
        AssetType, TICKER_PERFORMANCE_PERIODS, Ticker, TickerControl, TickerEmbedding,
        TickerHistory, TickerIndicator, TickerSentiment,
    },
    utils::data_utils::{
        calculate_performance, get_period_close, get_period_start, market_cap_label,
    },
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_decimal_macros::dec;
use std::collections::HashMap;
use tracing::{debug, error, warn};

use crate::stocks::{StocksService, indicators::IndicatorCalculator};
use anyhow::Result;

impl StocksService {
    // sync_ticker return true if not updated in 24 hours
    pub fn should_sync_ticker(&self, tc: &TickerControl) -> bool {
        if let Some(last_sync) = tc.last_sync_at {
            return Utc::now() - last_sync > Duration::hours(24);
        }
        true
    }

    // sync_history return true if not updated in 24 hours
    pub fn should_sync_history(&self, tc: &TickerControl) -> bool {
        if let Some(last_sync) = tc.last_history_sync_at {
            return Utc::now() - last_sync > Duration::hours(24);
        }
        true
    }

    // sync_sentiments return true if not updated in 24 hours
    pub fn should_sync_sentiments(&self, tc: &TickerControl) -> bool {
        if let Some(last_sync) = tc.last_sentiment_sync_at {
            return Utc::now() - last_sync > Duration::hours(24);
        }
        true
    }

    // sync_embeddings return true if not updated in 24 hours
    pub fn should_sync_embeddings(&self, tc: &TickerControl) -> bool {
        if let Some(last_sync) = tc.last_embedding_sync_at {
            return Utc::now() - last_sync > Duration::hours(24);
        }
        true
    }

    // sync_indicators return true if not updated in 24 hours
    pub fn should_sync_indicators(&self, tc: &TickerControl) -> bool {
        if let Some(last_sync) = tc.last_indicator_sync_at {
            return Utc::now() - last_sync > Duration::hours(24);
        }
        true
    }

    pub async fn update_and_save_single_ticker(
        &self,
        tc: &mut TickerControl,
        ticker: &mut Ticker,
    ) -> Result<()> {
        let update = true;
        // update single ticker
        // if self.should_sync_ticker(tc) {
            self.update_single_ticker(tc, ticker).await?;
        // }

        // update history
        if self.should_sync_history(tc) {
            match self.update_single_ticker_history(tc, ticker).await {
                Ok(new_histories) => {
                    if new_histories.len() > 0 {
                        debug!("Ticker New Histories: {}", new_histories.len());
                        tc.last_history_sync_at = Some(Utc::now());
                        if update {
                            self.storage_service.save_ticker_control(tc.clone()).await?;
                            self.storage_service
                                .save_ticker_history(&ticker.symbol, &new_histories)
                                .await?;
                        }
                    }
                }
                Err(e) => error!("History update failed for {}: {}", ticker.symbol, e),
            }
        }

        // update sentiments
        if self.should_sync_sentiments(tc) {
            match self.update_single_ticker_sentiments(tc, ticker).await {
                Ok(new_sentiments) => {
                    if new_sentiments.len() > 0 {
                        debug!("Ticker New Sentiments: {}", new_sentiments.len());
                        tc.last_sentiment_sync_at = Some(Utc::now());
                        if update {
                            self.storage_service.save_ticker_control(tc.clone()).await?;
                            self.storage_service
                                .save_ticker_sentiments(&ticker.symbol, &new_sentiments)
                                .await?;
                        }
                    }
                }
                Err(e) => error!("Sentiments update failed for {}: {}", ticker.symbol, e),
            }
        }

        if self.should_sync_embeddings(tc) {
            match self
                .update_single_ticker_sentiment_embeddings(tc, ticker)
                .await
            {
                Ok(new_embeddings) => {
                    if new_embeddings.len() > 0 {
                        debug!("Ticker New Embeddings: {}", new_embeddings.len());
                        tc.last_embedding_sync_at = Some(Utc::now());
                        if update {
                            self.storage_service.save_ticker_control(tc.clone()).await?;
                            self.storage_service
                                .save_ticker_embeddings(&ticker.symbol, &new_embeddings)
                                .await?;
                        }
                    }
                }
                Err(e) => error!("Embeddings update failed for {}: {}", ticker.symbol, e),
            }
        }

        //Get the history
        let mut histories = self
            .storage_service
            .get_ticker_history(&ticker.symbol)
            .await?;
        debug!("Ticker History: {}", histories.len());

        // update technical indicators
        if self.should_sync_indicators(tc) {
            match self
                .update_single_stock_indicators(tc, ticker, &histories)
                .await
            {
                Ok(new_indicators) => {
                    if new_indicators.len() > 0 {
                        debug!("Ticker New Indicators: {}", new_indicators.len());
                        tc.last_indicator_sync_at = Some(Utc::now());
                        if update {
                            self.storage_service.save_ticker_control(tc.clone()).await?;
                            self.storage_service
                                .save_ticker_indicators(&ticker.symbol, &new_indicators)
                                .await?;
                        }
                    }
                }
                Err(e) => error!("Indicators update failed for {}: {}", ticker.symbol, e),
            }
        }

        tc.last_sync_at = Some(Utc::now());

        // sort by descending for updating price history
        histories.sort_by(|a, b| b.date.cmp(&a.date));

        self.update_single_ticker_price_history(tc, ticker, &histories)
            .await?;
        //now calculate the performance
        self.update_single_ticker_performance(tc, ticker, &histories)
            .await?;

        //update signals
        self.update_single_ticker_signals(ticker).await?;

        debug!(
            "Ticker: {} market cap: {:?} price: {:?} analyst consensus: {:?}",
            ticker.symbol, ticker.market_cap, ticker.pr_last, ticker.analyst_consensus
        );

        // if update {
        self.storage_service.save_ticker(ticker.clone()).await?;
        self.storage_service.save_ticker_control(tc.clone()).await?;
        // }

        Ok(())
    }

    pub async fn update_single_ticker(
        &self,
        _tc: &mut TickerControl,
        ticker: &mut Ticker,
    ) -> Result<()> {
        match ticker.asset_type {
            AssetType::Stock => {
                let raw = self.provider_service.get_stock(&ticker.symbol).await?;
                ticker.update_from_alpha(raw);
            }
            // AssetType::Etf => {
            //     hists = Vec::new();
            // }
            AssetType::Crypto => {}

            _ => {}
        }

        Ok(())
    }

    pub async fn update_single_ticker_price_history(
        &self,
        _tc: &mut TickerControl,
        ticker: &mut Ticker,
        histories: &[TickerHistory],
    ) -> Result<()> {
        if histories.len() > 0 {
            let last_history = histories[0].clone();
            let mut prev_history = None;
            if histories.len() > 1 {
                prev_history = Some(histories[1].clone());
            }
            // update the price
            ticker.update_price_from_history(last_history, prev_history)?;
        }

        Ok(())
    }

    pub async fn update_single_ticker_performance(
        &self,
        _tc: &mut TickerControl,
        ticker: &mut Ticker,
        histories: &[TickerHistory],
    ) -> Result<()> {
        ticker.performance.clear();
        ticker.performance_search.clear();

        for period in TICKER_PERFORMANCE_PERIODS {
            if let Some(start_date) = get_period_start(period) {
                if let Some(period_close) = get_period_close(&histories, start_date) {
                    let mut period_map = HashMap::new();
                    let mut period_map_search = HashMap::new();
                    period_map.insert("price".to_string(), period_close);
                    period_map.insert(
                        "perc".to_string(),
                        calculate_performance(ticker.pr_close, period_close),
                    );

                    period_map_search
                        .insert("price".to_string(), period_close.to_f64().unwrap_or(0.0));
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
        }
        Ok(())
    }

    pub async fn update_single_ticker_history(
        &self,
        tc: &mut TickerControl,
        ticker: &mut Ticker,
    ) -> Result<Vec<TickerHistory>> {
        let Some(hist_start_date) = Utc::now().checked_sub_months(Months::new(60)) else {
            return Err(anyhow::anyhow!("Error calcuating start date"));
        };

        let histories = match ticker.asset_type {
            AssetType::Stock => {
                let thist = self
                    .provider_service
                    .get_stock_history(&ticker.symbol, &hist_start_date)
                    .await?;

                let histories =
                    TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?;
                histories
            }
            AssetType::Etf => {
                let thist = self
                    .provider_service
                    .get_stock_history(&ticker.symbol, &hist_start_date)
                    .await?;

                let histories =
                    TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?;
                histories
            }
            AssetType::Crypto => {
                let thist = self
                    .provider_service
                    .get_crypto_history(&ticker.symbol, "1day")
                    .await
                    .inspect_err(|e| {
                        warn!("Crypto Ticker history for '{}' error: {}", ticker.symbol, e)
                    })?;

                let histories =
                    TickerHistory::from_tiingo_batch(&ticker.symbol, &ticker.exchange, thist)?;
                histories
            }
        };

        debug!("Ticker History: {}", histories.len());
        let mut new_histories = Vec::new();
        if histories.len() > 0 {
            new_histories = match tc.last_history_sync_at {
                Some(last_sync) => histories
                    .into_iter()
                    .filter(|h| h.date > last_sync)
                    .collect(),
                None => {
                    // First sync - insert all
                    histories
                }
            };
        }

        Ok(new_histories)
    }

    pub async fn update_single_stock_indicators(
        &self,
        tc: &mut TickerControl,
        _ticker: &mut Ticker,
        histories: &[TickerHistory],
    ) -> Result<Vec<TickerIndicator>> {
        let mut new_indicators = Vec::new();

        if histories.len() > 0 {
            let sma_periods = &[20, 50, 100, 200];
            let ema_periods = &[12, 26, 50];
            let rsi_periods = &[10, 14, 26];
            let k_period = 14;
            let d_period = 3;
            let bb_period = 20;
            let bb_std_dev = 2.0;
            let atr_period = 14;

            let indicators = IndicatorCalculator::calculate_all_in_one_pass(
                &histories,
                sma_periods.to_vec(),
                ema_periods.to_vec(),
                rsi_periods.to_vec(),
                k_period,
                d_period,
                bb_period,
                bb_std_dev,
                atr_period,
            )?;

            debug!("Ticker Indicators updates: {}", indicators.len());
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
        }

        Ok(new_indicators)
    }

    pub async fn update_single_ticker_sentiments(
        &self,
        tc: &mut TickerControl,
        ticker: &mut Ticker,
    ) -> Result<Vec<TickerSentiment>> {
        let mut new_sentiments = Vec::new();

        let Some(date_from) = Utc::now().checked_sub_months(Months::new(6)) else {
            return Err(anyhow::anyhow!("Error with DateTime"));
        };
        let mut sentiments = Vec::new();
        let mut feeds_len = 0;
        match ticker.asset_type {
            AssetType::Stock => {
                let feeds = self
                    .provider_service
                    .get_ticker_sentiment(&ticker.symbol, &date_from)
                    .await?;
                feeds_len = feeds.len();
                sentiments = TickerSentiment::new_from_alpha_batch(&ticker.symbol, feeds);
            }
            _ => {}
        }
        debug!(
            "Ticker Feeds: {} Sentiments: {}",
            feeds_len,
            sentiments.len()
        );

        if sentiments.len() > 0 {
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

    pub async fn update_single_ticker_sentiment_embeddings(
        &self,
        tc: &mut TickerControl,
        ticker: &mut Ticker,
    ) -> Result<Vec<TickerEmbedding>> {
        let mut new_embeddings = Vec::new();
        let cmp_score = dec!(0.8);

        let sentiments = self
            .storage_service
            .get_ticker_sentiments_with_score(&ticker.symbol, &cmp_score)
            .await?;

        if sentiments.len() == 0 {
            return Ok(new_embeddings);
        }

        debug!(
            "Ticker Sentiments with score: {} - {}",
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

        let result = match self
            .embedding_client
            .embed_text_batch(&embedding_refs)
            .await
        {
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
        if embeddings.len() > 0 {
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

    pub async fn update_single_ticker_embedding(&self, ticker: &mut Ticker) -> Result<()> {
        // adding the industry twice to increase the weight.

        let market_cap_label = market_cap_label(ticker.market_cap);
        let overview_text = format!(
            "{} {} {} {} {}",
            ticker.name,
            ticker.sector.as_deref().unwrap_or(""),
            ticker.industry.as_deref().unwrap_or(""),
            market_cap_label,
            ticker.overview
        );

        match self.embedding_client.embed_text(&overview_text).await {
            Ok(embedding) => {
                ticker.overview_text = Some(overview_text);
                ticker.overview_embedding = Some(embedding.into_vec());
            }
            Err(e) => error!("Embedding failed for {}: {}", ticker.symbol, e),
        }

        let industry_text = ticker.industry.clone().unwrap_or("".to_string());
        match self.embedding_client.embed_text(&industry_text).await {
            Ok(embedding) => {
                ticker.industry_text = Some(industry_text);
                ticker.industry_embedding = Some(embedding.into_vec());
            }
            Err(e) => error!("Embedding failed for {}: {}", ticker.symbol, e),
        }

        self.storage_service.save_ticker(ticker.clone()).await?;

        Ok(())
    }

    pub async fn update_single_ticker_signals(&self, ticker: &mut Ticker) -> Result<()> {
        let indicators = self
            .storage_service
            .get_ticker_indicators_last_two(&ticker.symbol)
            .await?;

        if indicators.len() < 2 {
            return Ok(());
        }

        let prev = &indicators[0];
        let curr = &indicators[1];

        let mut signals = Vec::new();

        // Trend
        let sma_50 = curr.values.get("sma_50");
        let sma_200 = curr.values.get("sma_200");
        let prev_sma_50 = prev.values.get("sma_50");
        let prev_sma_200 = prev.values.get("sma_200");

        if let (Some(s50), Some(s200), Some(ps50), Some(ps200)) =
            (sma_50, sma_200, prev_sma_50, prev_sma_200)
        {
            if s50 > s200 && ps50 <= ps200 {
                signals.push("Golden Cross".to_string());
            }
            if s50 < s200 && ps50 >= ps200 {
                signals.push("Death Cross".to_string());
            }
            if s50 > s200 {
                signals.push("Above SMA50".to_string());
            }
            if s50 < s200 {
                signals.push("Below SMA50".to_string());
            }
        }

        // MACD crossover
        let macd = curr.values.get("macd");
        let signal = curr.values.get("macd_signal");
        let prev_macd = prev.values.get("macd");
        let prev_signal = prev.values.get("macd_signal");

        if let (Some(m), Some(s), Some(pm), Some(ps)) = (macd, signal, prev_macd, prev_signal) {
            if m > s && pm <= ps {
                signals.push("MACD Bullish Crossover".to_string());
            }
            if m < s && pm >= ps {
                signals.push("MACD Bearish Crossover".to_string());
            }
        }

        // RSI
        if let Some(rsi) = curr.values.get("rsi_14") {
            if rsi < &Decimal::from(30) {
                signals.push("RSI Oversold".to_string());
            }
            if rsi > &Decimal::from(70) {
                signals.push("RSI Overbought".to_string());
            }
        }

        // Bollinger Bands
        let price = Decimal::try_from(ticker.pr_last)?;
        if let (Some(upper), Some(lower)) =
            (curr.values.get("bb_upper"), curr.values.get("bb_lower"))
        {
            if &price > upper {
                signals.push("BB Breakout Upper".to_string());
            }
            if &price < lower {
                signals.push("BB Breakout Lower".to_string());
            }
            let width = upper - lower;
            // BB squeeze — bands narrower than threshold
            if width < Decimal::from(10) {
                signals.push("BB Squeeze".to_string());
            }
        }

        // Stochastic
        let k = curr.values.get("stochastic_k_14");
        let d = curr.values.get("stochastic_d");
        let prev_k = prev.values.get("stochastic_k_14");
        let prev_d = prev.values.get("stochastic_d");

        if let (Some(k), Some(d), Some(pk), Some(pd)) = (k, d, prev_k, prev_d) {
            if k > d && pk <= pd {
                signals.push("Stochastic Bullish".to_string());
            }
            if k < d && pk >= pd {
                signals.push("Stochastic Bearish".to_string());
            }
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

        ticker.signals = signals;
        Ok(())
    }
}
