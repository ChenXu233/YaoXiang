//! HTTP 下载（RFC-014b Phase 4a/5d 共用：GitHub 适配层与 `[binaries]` 预编译）
//!
//! GET 幂等，瞬时故障（429/5xx）退避重试；POST 类非幂等请求不在此层。

use std::path::Path;
use std::time::Duration;

use crate::package::error::{PackageError, PackageResult};

/// 429/5xx 的最大重试次数（退避基数递增，Retry-After 优先）
pub(crate) const MAX_RETRIES: u32 = 3;

/// 构造 reqwest 客户端；`no_proxy` 供测试绕过系统代理直连本地 mock
pub(crate) fn client(no_proxy: bool) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(format!("yaoxiang-pm/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60));
    if no_proxy {
        builder = builder.no_proxy();
    }
    builder.build().expect("build reqwest client")
}

/// 下载文件到 `dest`；瞬时故障按 `backoff_base` 指数退避（Retry-After 优先）
pub(crate) async fn download_file(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    dest: &Path,
    timeout: Duration,
    backoff_base: Duration,
) -> PackageResult<()> {
    let mut attempt = 0u32;
    loop {
        let mut req = client.get(url);
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| PackageError::Network(e.to_string()))?;
        let status = resp.status();

        if status.is_success() {
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| PackageError::Network(e.to_string()))?;
            std::fs::write(dest, &bytes)?;
            return Ok(());
        }

        if attempt < MAX_RETRIES && (status.as_u16() == 429 || status.is_server_error()) {
            let delay = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or_else(|| backoff_base * 2u32.pow(attempt));
            tokio::time::sleep(delay).await;
            attempt += 1;
            continue;
        }

        return Err(PackageError::Network(format!(
            "download failed: {status} ({url})"
        )));
    }
}
