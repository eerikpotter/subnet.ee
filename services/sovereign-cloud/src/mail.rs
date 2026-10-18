//! Resend transport only. Delivery results never establish identity or file access.
use ic_cdk_management_canister::{HttpMethod, HttpRequest};
use serde::{Deserialize, Serialize};
use sovereign_cloud::email::DeliveryStatus;

#[derive(Clone)]
pub struct MailConfig {
    pub api_key: String,
    pub from: String,
    pub origin: String,
}
pub fn configuration() -> Result<Option<MailConfig>, String> {
    let read = |key| if ic_cdk::api::env_var_name_exists(key) { ic_cdk::api::env_var_value(key) } else { String::new() };
    let api_key = read("RESEND_API_KEY");
    if api_key.is_empty() {
        return Ok(None);
    }
    let config = MailConfig { api_key, from: read("SUBNET_MAIL_FROM"), origin: read("SUBNET_MAIL_ORIGIN") };
    config.validate()?;
    Ok(Some(config))
}
impl MailConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.api_key.starts_with("re_")
            || !(24..=200).contains(&self.api_key.len())
            || !self.api_key.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err("Invalid mail credential".into());
        }
        if self.from != "subnet.ee <info@subnet.ee>" {
            return Err("Invalid mail sender".into());
        }
        let local = self.origin.strip_prefix("http://").and_then(|s| s.rsplit_once(':')).is_some_and(|(host, port)| {
            host.ends_with(".localhost") && host.bytes().all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b)) && port.parse::<u16>().is_ok_and(|p| p > 0)
        });
        if self.origin != "https://www.subnet.ee" && !local {
            return Err("Invalid application origin".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MailPayload {
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub text: String,
}

// Runtime-only claim. Credentials are never serialized with the durable outbox.
pub struct DeliveryClaim {
    pub attempt: u8,
    pub key: String,
    pub payload: Vec<u8>,
    pub idempotency: String,
}

pub async fn send(key: &str, idempotency: &str, payload: Vec<u8>) -> DeliveryStatus {
    let request = HttpRequest::new("https://api.resend.com/emails")
        .with_method(HttpMethod::POST)
        .non_replicated()
        .with_header("Authorization", format!("Bearer {key}"))
        .with_header("Content-Type", "application/json")
        .with_header("Idempotency-Key", idempotency)
        .with_body(payload)
        .with_max_response_bytes(16_384)
        .with_expected_roundtrip_time_ms(30_000);
    match request.send().await {
        Ok(response) => classify(response.status.to_string().parse().unwrap_or(0), &response.body),
        // Do not expose provider bodies, headers or reject text, which may echo secrets.
        Err(_) => DeliveryStatus::Unknown,
    }
}

pub fn classify(status: u16, body: &[u8]) -> DeliveryStatus {
    if (200..300).contains(&status) {
        let valid = serde_json::from_slice::<serde_json::Value>(body).ok().and_then(|v| v["id"].as_str().map(|id| !id.is_empty())).unwrap_or(false);
        return if valid { DeliveryStatus::Accepted } else { DeliveryStatus::Unknown };
    }
    let name = serde_json::from_slice::<serde_json::Value>(body).ok().and_then(|v| v["name"].as_str().map(str::to_owned)).unwrap_or_default();
    if status == 429 || (status == 409 && name == "concurrent_idempotent_requests") {
        DeliveryStatus::Retryable
    } else if status >= 500 || status == 0 {
        DeliveryStatus::Unknown
    } else {
        DeliveryStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delivery_outcomes_do_not_confuse_acceptance_with_delivery_or_retry_bad_payloads() {
        assert_eq!(classify(200, br#"{"id":"message"}"#), DeliveryStatus::Accepted);
        assert_eq!(classify(200, b"bad body"), DeliveryStatus::Unknown);
        assert_eq!(classify(409, br#"{"name":"concurrent_idempotent_requests"}"#), DeliveryStatus::Retryable);
        assert_eq!(classify(409, br#"{"name":"invalid_idempotent_request"}"#), DeliveryStatus::Failed);
        assert_eq!(classify(429, b""), DeliveryStatus::Retryable);
        assert_eq!(classify(503, b""), DeliveryStatus::Unknown);
        assert_eq!(classify(401, b""), DeliveryStatus::Failed);
    }
}
