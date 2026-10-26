//! HTTP fetching with limits and a resolver that refuses private networks.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect;
use url::Url;

use crate::ImportError;

#[derive(Debug, Clone)]
pub struct FetchLimits {
    pub max_bytes: usize,
    pub timeout: Duration,
    pub max_redirects: usize,
    /// Allow loopback and private addresses (tests and self-hosted setups only).
    pub allow_private: bool,
}

impl Default for FetchLimits {
    fn default() -> Self {
        Self {
            max_bytes: 10 * 1024 * 1024,
            timeout: Duration::from_secs(15),
            max_redirects: 5,
            allow_private: false,
        }
    }
}

pub struct Fetched {
    pub final_url: Url,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// True for addresses an imported link must never reach: loopback, private,
/// link-local, unspecified, multicast, carrier-grade NAT and unique-local ranges.
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || o[0] == 0
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_forbidden_ip(IpAddr::V4(v4));
            }
            let seg = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg[0] & 0xfe00) == 0xfc00
                || (seg[0] & 0xffc0) == 0xfe80
        }
    }
}

fn host_is_forbidden(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => is_forbidden_ip(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_forbidden_ip(IpAddr::V6(ip)),
        Some(url::Host::Domain(d)) => {
            d.eq_ignore_ascii_case("localhost") || d.ends_with(".localhost")
        }
        None => true,
    }
}

/// DNS resolver that drops forbidden addresses, so redirects cannot reach them either.
struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|a| !is_forbidden_ip(a.ip()))
                .collect();
            if addrs.is_empty() {
                return Err(format!("{host} resolves only to private or local addresses").into());
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

pub fn client(limits: &FetchLimits) -> Result<reqwest::Client, ImportError> {
    let allow_private = limits.allow_private;
    let max_redirects = limits.max_redirects;
    let policy = redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= max_redirects {
            attempt.error("too many redirects")
        } else if !matches!(attempt.url().scheme(), "http" | "https") {
            attempt.error("redirect to a non-web link")
        } else if !allow_private && host_is_forbidden(attempt.url()) {
            attempt.error("redirect to a private address")
        } else {
            attempt.follow()
        }
    });
    let mut builder = reqwest::Client::builder()
        .timeout(limits.timeout)
        .redirect(policy)
        .user_agent(concat!(
            "JmdEditor/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/luckySpro/JmdEditor)"
        ));
    if !allow_private {
        builder = builder.dns_resolver(Arc::new(PublicOnlyResolver));
    }
    builder
        .build()
        .map_err(|e| ImportError::Network(e.to_string()))
}

/// The error with its causes, so "error sending request" says why.
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !msg.contains(&text) {
            msg.push_str(": ");
            msg.push_str(&text);
        }
        source = cause.source();
    }
    msg
}

/// Downloads `url` within the limits, streaming so oversized bodies stop early.
pub async fn fetch(
    client: &reqwest::Client,
    url: &Url,
    limits: &FetchLimits,
) -> Result<Fetched, ImportError> {
    if !limits.allow_private && host_is_forbidden(url) {
        return Err(ImportError::Forbidden(
            url.host_str().unwrap_or_default().to_string(),
        ));
    }
    let mut resp = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| ImportError::Network(error_chain(&e)))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ImportError::Status(status.as_u16()));
    }
    if let Some(len) = resp.content_length() {
        if len as usize > limits.max_bytes {
            return Err(ImportError::TooLarge(limits.max_bytes));
        }
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let final_url = resp.url().clone();
    let mut body = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| ImportError::Network(e.to_string()))?
    {
        if body.len() + chunk.len() > limits.max_bytes {
            return Err(ImportError::TooLarge(limits.max_bytes));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Fetched {
        final_url,
        content_type,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbids_private_ranges() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.1",
            "172.16.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(is_forbidden_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["93.184.216.34", "140.82.112.3", "2606:4700::1111"] {
            assert!(!is_forbidden_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn forbids_localhost_names() {
        assert!(host_is_forbidden(
            &Url::parse("http://localhost:8080/").unwrap()
        ));
        assert!(host_is_forbidden(
            &Url::parse("http://api.localhost/").unwrap()
        ));
        assert!(host_is_forbidden(&Url::parse("http://[::1]/").unwrap()));
        assert!(!host_is_forbidden(
            &Url::parse("https://github.com/").unwrap()
        ));
    }
}
