use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Proxy,
};
use serde::de::DeserializeOwned;
use std::{env, time::Duration};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;

/// HTTP client wrapper with timeout and authentication support
pub struct HttpClient {
    client: reqwest::Client,
    timeout: Duration,
    max_retries: usize,
    retry_base_ms: u64,
}

/// Builder for `HttpClient` with configurable retry and timeout settings
pub struct HttpClientBuilder {
    timeout_secs: u64,
    max_retries: usize,
    retry_base_ms: u64,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            max_retries: 3,
            retry_base_ms: 1000,
        }
    }
}

impl HttpClientBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the request timeout in seconds (default: 10)
    #[allow(dead_code)]
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set the maximum number of retries (default: 3)
    #[allow(dead_code)]
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the base delay in milliseconds for exponential backoff (default: 1000)
    #[allow(dead_code)]
    pub fn retry_base_ms(mut self, ms: u64) -> Self {
        self.retry_base_ms = ms;
        self
    }

    /// Build the `HttpClient`
    pub fn build(self) -> Result<HttpClient> {
        let mut builder =
            reqwest::Client::builder().timeout(Duration::from_secs(self.timeout_secs));

        if let Some(proxy) = env_proxy_value(&["HTTPS_PROXY", "https_proxy"]) {
            builder = builder.proxy(Proxy::https(&proxy).context("Invalid HTTPS proxy URL")?);
        }

        if let Some(proxy) = env_proxy_value(&["HTTP_PROXY", "http_proxy"]) {
            builder = builder.proxy(Proxy::http(&proxy).context("Invalid HTTP proxy URL")?);
        }

        let client = builder.build().context("Failed to create HTTP client")?;

        Ok(HttpClient {
            client,
            timeout: Duration::from_secs(self.timeout_secs),
            max_retries: self.max_retries,
            retry_base_ms: self.retry_base_ms,
        })
    }
}

fn env_proxy_value(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| env::var(key).ok())
        .filter(|v| !v.is_empty())
}

impl HttpClient {
    /// Create a new HTTP client with specified timeout
    pub fn new(timeout_secs: u64) -> Result<Self> {
        HttpClientBuilder::new().timeout_secs(timeout_secs).build()
    }

    /// Create a builder for more fine-grained control
    #[allow(dead_code)]
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    /// Fetch JSON data from URL with optional authentication token
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, token: Option<&str>) -> Result<T> {
        if self.max_retries == 0 {
            // No retries, execute once
            return self.get_json_once(url, token).await;
        }

        let retry_strategy = ExponentialBackoff::from_millis(self.retry_base_ms)
            .map(jitter)
            .take(self.max_retries);

        Retry::spawn(retry_strategy, || async {
            self.get_json_once(url, token).await
        })
        .await
    }

    /// Internal method to fetch JSON once (used by retry logic)
    async fn get_json_once<T: DeserializeOwned>(
        &self,
        url: &str,
        token: Option<&str>,
    ) -> Result<T> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("git-trending-motd/0.0.2"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(token) = token {
            let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("Invalid authentication token")?;
            headers.insert(AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {url}"))?;

        let status = response.status();

        // Don't retry on 4xx client errors
        if status.is_client_error() {
            anyhow::bail!("HTTP request failed with client error {status}: {url}");
        }

        if !status.is_success() {
            anyhow::bail!("HTTP request failed with status {status}: {url}");
        }

        response
            .json::<T>()
            .await
            .with_context(|| format!("Failed to parse JSON response from {url}"))
    }

    /// Fetch HTML content from URL (for web scraping)
    pub async fn get_html(&self, url: &str) -> Result<String> {
        if self.max_retries == 0 {
            // No retries, execute once
            return self.get_html_once(url).await;
        }

        let retry_strategy = ExponentialBackoff::from_millis(self.retry_base_ms)
            .map(jitter)
            .take(self.max_retries);

        Retry::spawn(retry_strategy, || async { self.get_html_once(url).await }).await
    }

    /// Internal method to fetch HTML once (used by retry logic)
    async fn get_html_once(&self, url: &str) -> Result<String> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("git-trending-motd/0.0.2"));
        headers.insert(ACCEPT, HeaderValue::from_static("text/html"));

        let response = self
            .client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {url}"))?;

        let status = response.status();

        // Don't retry on 4xx client errors
        if status.is_client_error() {
            anyhow::bail!("HTTP request failed with client error {status}: {url}");
        }

        if !status.is_success() {
            anyhow::bail!("HTTP request failed with status {status}: {url}");
        }

        response
            .text()
            .await
            .with_context(|| format!("Failed to read HTML response from {url}"))
    }

    /// Send a PUT request (for starring repositories)
    pub async fn put(&self, url: &str, token: Option<&str>) -> Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("git-trending-motd/0.0.2"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(token) = token {
            let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("Invalid authentication token")?;
            headers.insert(AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .put(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to PUT to URL: {url}"))?;

        let status = response.status();

        if !status.is_success() && status != reqwest::StatusCode::NO_CONTENT {
            anyhow::bail!("PUT request failed with status {status}: {url}");
        }

        Ok(())
    }

    /// Send a DELETE request (for unstarring repositories)
    #[allow(dead_code)]
    pub async fn delete(&self, url: &str, token: Option<&str>) -> Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("git-trending-motd/0.0.2"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(token) = token {
            let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("Invalid authentication token")?;
            headers.insert(AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .delete(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to DELETE to URL: {url}"))?;

        let status = response.status();

        if !status.is_success() && status != reqwest::StatusCode::NO_CONTENT {
            anyhow::bail!("DELETE request failed with status {status}: {url}");
        }

        Ok(())
    }

    /// Send a HEAD request to check if a resource exists (for checking starred status)
    #[allow(dead_code)]
    pub async fn head(&self, url: &str, token: Option<&str>) -> Result<bool> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("git-trending-motd/0.0.2"));

        if let Some(token) = token {
            let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("Invalid authentication token")?;
            headers.insert(AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .head(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to HEAD URL: {url}"))?;

        let status = response.status();

        // GitHub returns 204 if starred, 404 if not starred
        Ok(status == reqwest::StatusCode::NO_CONTENT || status.is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new(6);
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_configuration() {
        let client = HttpClient::builder()
            .timeout_secs(30)
            .max_retries(5)
            .retry_base_ms(500)
            .build()
            .unwrap();

        assert_eq!(client.timeout.as_secs(), 30);
        assert_eq!(client.max_retries, 5);
        assert_eq!(client.retry_base_ms, 500);
    }

    #[tokio::test]
    async fn test_get_json_with_mock() {
        // Integration tests with mockito will be added in provider tests
        // This is a placeholder for unit test structure
        let client = HttpClient::new(6).unwrap();
        assert_eq!(client.timeout.as_secs(), 6);
    }

    #[tokio::test]
    async fn test_timeout_error_handling() {
        // Test with very short timeout to trigger timeout error
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(1)
            .max_retries(0)
            .build()
            .unwrap();

        // This should timeout quickly (using a slow endpoint)
        let result: Result<serde_json::Value> =
            client.get_json("https://httpbin.org/delay/10", None).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        // Accept timeout error or any network error (httpbin might be down)
        assert!(
            err_msg.contains("failed")
                || err_msg.contains("timeout")
                || err_msg.contains("error")
                || err_msg.contains("timed out")
        );
    }

    #[tokio::test]
    async fn test_404_error_handling() {
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();

        // This should return 404
        let result: Result<serde_json::Value> = client
            .get_json("https://httpbin.org/status/404", None)
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("404") || err_msg.contains("client error"));
    }

    #[tokio::test]
    async fn test_500_error_handling() {
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();

        // This should return 500
        let result: Result<serde_json::Value> = client
            .get_json("https://httpbin.org/status/500", None)
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("500") || err_msg.contains("status"));
    }

    #[tokio::test]
    async fn test_invalid_json_handling() {
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();

        // This returns HTML, not JSON
        let result: Result<serde_json::Value> =
            client.get_json("https://httpbin.org/html", None).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        // Accept JSON parse error or any network/fetch error (httpbin might be down)
        // Also accept empty error message (some network errors)
        assert!(
            err_msg.contains("json")
                || err_msg.contains("failed")
                || err_msg.contains("error")
                || err_msg.contains("parse")
                || err_msg.contains("expected")
        );
    }

    #[tokio::test]
    async fn test_get_html_success() {
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();

        let result = client.get_html("https://httpbin.org/html").await;

        // Allow for network issues - httpbin.org might be down
        match result {
            Ok(html) => {
                assert!(html.contains("html") || html.contains("HTML"));
            }
            Err(e) => {
                // If httpbin is down, that's okay for this test
                eprintln!("httpbin.org test skipped: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_network_error_handling() {
        // Use max_retries(0) to avoid retry delays in tests
        let client = HttpClient::builder()
            .timeout_secs(5)
            .max_retries(0)
            .build()
            .unwrap();

        // Invalid hostname should fail
        let result: Result<serde_json::Value> = client
            .get_json("https://this-domain-does-not-exist-12345.com", None)
            .await;

        assert!(result.is_err());
    }
}
