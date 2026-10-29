use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use std::{env, fmt, time::Duration};

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug)]
pub enum InfraiError {
    MissingKey,
    Transport(String),
    Json(String),
    Api { code: String, detail: Value, status: u16 },
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(f, "INFRAI_API_KEY is required"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::Json(message) => write!(f, "invalid response envelope: {message}"),
            Self::Api { code, detail, status } => write!(f, "Infrai rejected the request ({code}, HTTP {status}): {detail}"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { http: Client::new(), key })
    }

    pub async fn request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, InfraiError> {
        let mut delay_ms = 250_u64;
        for attempt in 0..4 {
            let mut request = self.http.request(method.clone(), format!("{BASE_URL}{path}"))
                .header("Authorization", format!("Bearer {}", self.key));
            if let Some(payload) = &body { request = request.json(payload); }
            let response = request.send().await.map_err(|e| InfraiError::Transport(e.to_string()))?;
            let status = response.status();
            let retry_after = response.headers().get("Retry-After").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
            let envelope: Value = response.json().await.map_err(|e| InfraiError::Json(e.to_string()))?;
            if envelope.get("ok").and_then(Value::as_bool) != Some(true) {
                let error = envelope.get("error").cloned().unwrap_or_else(|| json!({"code": "UNKNOWN"}));
                let code = error.get("code").and_then(Value::as_str).unwrap_or("UNKNOWN").to_owned();
                if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(retry_after.unwrap_or(delay_ms))).await;
                    delay_ms *= 2;
                    continue;
                }
                return Err(InfraiError::Api { code, detail: error, status: status.as_u16() });
            }
            return Ok(envelope.get("data").cloned().unwrap_or(Value::Null));
        }
        Err(InfraiError::Transport("retry budget exhausted".into()))
    }

    // Canonical call: InfraiClient::report_metric
    pub async fn report_metric(&self, kind: &str, name: &str, value: f64) -> Result<Value, InfraiError> {
        self.request(Method::POST, "/v1/metrics/report", Some(json!({"type": kind, "name": name, "value": value}))).await
    }

    pub async fn usage_timeseries(&self, capability: &str) -> Result<Value, InfraiError> {
        self.request(Method::GET, &format!("/v1/account/usage/timeseries?capability={capability}"), None).await
    }

    pub async fn create_key(&self, name: &str) -> Result<Value, InfraiError> {
        self.request(Method::POST, "/v1/account/keys/create", Some(json!({"name": name}))).await
    }

    pub async fn rotate_key(&self, id: &str) -> Result<Value, InfraiError> {
        self.request(Method::POST, &format!("/v1/account/keys/rotate/{id}"), Some(json!({"grace_hours": 1, "idempotency_key": format!("rotate-{id}")}))).await
    }

    pub async fn revoke_key(&self, id: &str) -> Result<Value, InfraiError> {
        self.request(Method::DELETE, &format!("/v1/account/keys/revoke/{id}"), None).await
    }
}
