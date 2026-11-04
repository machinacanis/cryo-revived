//! QQ Bot OpenAPI REST client.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::AUTHORIZATION;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::Result;

pub(crate) const BASE_URL: &str = "https://api.bot.qq.com";

/// Business failure returned by the platform (`err_code`/`code` != 0),
/// or an HTTP-level error with a platform trace id.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: i64,
    pub message: String,
    pub trace_id: String,
    pub status: u16,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "qqbot api: err_code={} status={} msg={} trace={}",
            self.code, self.status, self.message, self.trace_id
        )
    }
}

impl std::error::Error for ApiError {}

struct Cached {
    token: String,
    expires: Instant,
}

struct Inner {
    app_id: String,
    secret: String,
    http: reqwest::Client,
    token: Mutex<Option<Cached>>,
}

/// Calls the OpenAPI. Cheap to clone — clones share the token cache.
/// Obtain one via [`crate::Bot::api`].
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    pub fn new(app_id: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Client {
            inner: Arc::new(Inner {
                app_id: app_id.into(),
                secret: app_secret.into(),
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .expect("reqwest client"),
                token: Mutex::new(None),
            }),
        }
    }

    /// A valid access_token, refreshing when needed. Tokens are cached;
    /// the platform returns the same token until it nears expiry.
    pub async fn token(&self) -> Result<String> {
        let mut guard = self.inner.token.lock().await;
        if let Some(c) = &*guard {
            if c.expires > Instant::now() + Duration::from_secs(60) {
                return Ok(c.token.clone());
            }
        }
        let out: Value = self
            .raw(
                Method::POST,
                format!("{BASE_URL}/app/getAppAccessToken"),
                None,
                &[],
                Some(&serde_json::json!({
                    "appId": self.inner.app_id,
                    "clientSecret": self.inner.secret,
                })),
            )
            .await?;
        let token = out["access_token"].as_str().unwrap_or_default().to_string();
        if token.is_empty() {
            return Err(ApiError {
                code: 0,
                message: "empty access_token".into(),
                trace_id: String::new(),
                status: 0,
            }
            .into());
        }
        let secs = out["expires_in"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| out["expires_in"].as_u64())
            .unwrap_or(7200);
        *guard = Some(Cached {
            token: token.clone(),
            expires: Instant::now() + Duration::from_secs(secs),
        });
        Ok(token)
    }

    /// Authenticated request, decoded into `T`. Business errors surface as
    /// [`ApiError`] even when the HTTP status is 200.
    pub(crate) async fn do_req<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&(impl Serialize + Sync)>,
    ) -> Result<T> {
        let tok = self.token().await?;
        let out = self
            .raw(method, format!("{BASE_URL}{path}"), Some(tok), query, body)
            .await?;
        Ok(serde_json::from_value(out)?)
    }

    /// GET helper.
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        self.do_req::<T>(Method::GET, path, query, None::<&Value>)
            .await
    }

    /// POST helper.
    pub(crate) async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &(impl Serialize + Sync),
    ) -> Result<T> {
        self.do_req(Method::POST, path, &[], Some(body)).await
    }

    /// DELETE helper; ignores the response body.
    pub(crate) async fn delete(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        self.do_req::<serde::de::IgnoredAny>(Method::DELETE, path, query, None::<&Value>)
            .await?;
        Ok(())
    }

    /// PUT helper; ignores the response body.
    pub(crate) async fn put(&self, path: &str, body: &(impl Serialize + Sync)) -> Result<()> {
        self.do_req::<serde::de::IgnoredAny>(Method::PUT, path, &[], Some(body))
            .await?;
        Ok(())
    }

    async fn raw(
        &self,
        method: Method,
        url: String,
        auth: Option<String>,
        query: &[(&str, &str)],
        body: Option<&(impl Serialize + Sync)>,
    ) -> Result<Value> {
        let mut req = self.inner.http.request(method, url).query(query);
        if let Some(tok) = auth {
            req = req.header(AUTHORIZATION, format!("QQBot {tok}"));
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let trace = resp
            .headers()
            .get("X-Tps-trace-ID")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let data: Value = resp.json().await.unwrap_or(Value::Null);
        if let Some(e) = envelope_error(&data, status, trace) {
            return Err(e.into());
        }
        Ok(data)
    }
}

/// Inspects a response body for the platform's business-error envelope:
/// `err_code` (OpenAPI) or `code` (token/api errors), plus trace id.
fn envelope_error(data: &Value, status: u16, header_trace: String) -> Option<ApiError> {
    let code = data["err_code"].as_i64().unwrap_or(0) + data["code"].as_i64().unwrap_or(0);
    let message = data["message"].as_str().unwrap_or_default().to_string();
    let trace_id = match data["trace_id"].as_str() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => header_trace,
    };
    if code != 0 {
        return Some(ApiError {
            code,
            message,
            trace_id,
            status,
        });
    }
    if status >= 400 {
        return Some(ApiError {
            code: -(status as i64),
            message: if message.is_empty() {
                data.to_string()
            } else {
                message
            },
            trace_id,
            status,
        });
    }
    None
}
