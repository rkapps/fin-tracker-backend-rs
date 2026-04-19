use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AlphaTicker {
    pub symbol: String,
    pub asset_type: String,
    pub name: String,
    pub description: String,
    pub exchange: String,
    pub country: String,
    pub currency: String,
    pub sector: String,
    pub industry: String,
    pub market_capitalization: String,
    #[serde(rename = "PERatio")]
    pub peratio: String,
    #[serde(rename = "PEGRatio")]
    pub pegratio: String,
    #[serde(rename = "PriceToBookRatio")]
    pub pbratio: String,
    #[serde(rename = "PriceToSalesRatioTTM")]
    pub psratio: String,
    pub book_value: String,
    pub dividend_per_share: String,
    pub dividend_yield: String,
    #[serde(rename = "EPS")]
    pub eps: String,
    pub dividend_date: String,
    pub ex_dividend_date: String,

    #[serde(rename = "ForwardPE")]
    pub foward_pe: String,
    pub beta: String,
    pub profit_margin: String,
    #[serde(rename = "OperatingMarginTTM")]
    pub operating_margin_ttm: String,
    #[serde(rename = "ReturnOnEquityTTM")]
    pub return_on_equity_ttm: String,
    #[serde(rename = "ReturnOnAssetsTTM")]
    pub return_on_asset_ttm: String,
    #[serde(rename = "EBITDA")]
    pub ebitda: String,
    #[serde(rename = "EVToRevenue")]
    pub ev_to_revenue: String,
    #[serde(rename = "EVToEBITDA")]
    pub ev_to_ebitda: String,
    pub shares_outstanding: String,
    pub shares_float: String,
    pub percent_insiders: String,
    pub percent_institutions: String,

    #[serde(rename = "QuarterlyEarningsGrowthYOY")]
    pub quarterly_earnings_growth_yoy: String,
    #[serde(rename = "QuarterlyRevenueGrowthYOY")]
    pub quarterly_revenue_growth_yoy: String,
    pub analyst_target_price: String,
    pub analyst_rating_strong_buy: String,
    pub analyst_rating_buy: String,
    pub analyst_rating_hold: String,
    pub analyst_rating_sell: String,
    pub analyst_rating_strong_sell: String,

    #[serde(rename = "52WeekHigh")]
    pub pr_52_wk_high: String,
    #[serde(rename = "52WeekLow")]
    pub pr_52_wk_low: String,
}

#[derive(Debug, Deserialize)]
pub struct AlphaEtf {
    pub net_assets: String,
    pub net_expense_ratio: String,
    pub dividend_yield: String,
    pub inception_date: String,
}

#[derive(Debug, Deserialize)]
pub struct AlphaTickerSentiment {
    pub feed: Vec<AlphaTickerSentimentFeed>,
}

#[derive(Debug, Deserialize)]
pub struct AlphaTickerSentimentFeed {
    pub title: String,
    pub url: String,
    pub time_published: String,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub source: String,
    pub category_within_source: String,
    pub source_domain: String,
    pub ticker_sentiment: Vec<AlphTickerSentimentFeedTickerSentiment>,
}

#[derive(Debug, Deserialize)]
pub struct AlphTickerSentimentFeedTickerSentiment {
    pub ticker: String,
    pub relevance_score: String,
    pub ticker_sentiment_score: String,
    pub ticker_sentiment_label: String,
}
