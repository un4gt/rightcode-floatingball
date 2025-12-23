use serde::Deserialize;

use crate::config::{AppConfig, normalize_bearer_token, normalize_cookie_header_value};

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subscription {
    pub name: String,
    pub total_quota: f64,
    pub remaining_quota: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("missing config: bearer token or cf_clearance cookie")]
    MissingConfig,
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
}

pub async fn fetch_subscriptions(config: &AppConfig) -> Result<SubscriptionsResponse, FetchError> {
    if config.bearer_token.trim().is_empty() || config.cookie.trim().is_empty() {
        return Err(FetchError::MissingConfig);
    }

    let user_agent = config.user_agent.trim();
    let client = reqwest::Client::builder()
        .user_agent(if user_agent.is_empty() {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:146.0) Gecko/20100101 Firefox/146.0"
        } else {
            user_agent
        })
        .build()?;

    let base = config.api_base.trim_end_matches('/');
    let url = format!("{base}/subscriptions/list");

    let token = normalize_bearer_token(&config.bearer_token);
    let cookie = normalize_cookie_header_value(&config.cookie);

    let response = client
        .get(url)
        .header("Accept", "*/*")
        .header("Referer", format!("{base}/dashboard"))
        .header("Content-Type", "application/json")
        .header("Authorization", token)
        .header("Cookie", cookie)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json::<SubscriptionsResponse>().await?)
}

pub fn default_subscription_index(
    subscriptions: &[Subscription],
    preferred_name: &str,
) -> Option<usize> {
    let preferred = subscriptions
        .iter()
        .position(|s| s.name.trim() == preferred_name.trim() && s.remaining_quota > 0.0);

    if preferred.is_some() {
        return preferred;
    }

    subscriptions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.total_quota > 0.0)
        .max_by(|(_, a), (_, b)| {
            a.remaining_quota
                .partial_cmp(&b.remaining_quota)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
}

pub fn remaining_ratio(subscription: &Subscription) -> f32 {
    if subscription.total_quota <= 0.0 {
        return 0.0;
    }
    let ratio = subscription.remaining_quota / subscription.total_quota;
    ratio.clamp(0.0, 1.0) as f32
}
