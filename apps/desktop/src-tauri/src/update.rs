//! Required upgrades.
//!
//! Every release carries `policy.json` (latest version, lowest version still
//! allowed to run, notes) and `policy.json.sig`, signed with the same minisign
//! key as the updater packages. Only a policy whose signature checks out can
//! lock the editor. The highest verified minimum is remembered, so an offline
//! start or a stale mirror cannot lower it, and a failed check never forces
//! anything by itself.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const RELEASES: &str = "https://github.com/luckySpro/JmdEditor/releases/latest/download";
/// Transport mirror for mainland China; the signature is what makes it safe.
const MIRROR: &str =
    "https://gh-proxy.com/https://github.com/luckySpro/JmdEditor/releases/latest/download";
const MAX_POLICY_BYTES: usize = 64 * 1024;

/// Where `policy.json` and `latest.json` are fetched from. `JMD_UPDATE_BASE`
/// points a test build at a local folder of release files.
pub fn bases() -> Vec<String> {
    match std::env::var("JMD_UPDATE_BASE") {
        Ok(base) if !base.trim().is_empty() => vec![base.trim().trim_end_matches('/').to_string()],
        _ => vec![RELEASES.to_string(), MIRROR.to_string()],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub version: String,
    pub minimum: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
}

/// What the UI shows. `Unknown` means no check succeeded and nothing verified
/// is cached: the editor stays fully usable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateStatus {
    UpToDate {
        current: String,
    },
    Optional {
        current: String,
        version: String,
        notes: String,
    },
    Required {
        current: String,
        version: String,
        minimum: String,
        notes: String,
    },
    Unknown {
        current: String,
        reason: String,
    },
}

fn base64_text(b64: &str) -> Result<String, AppError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| AppError::new("update_signature", e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| AppError::new("update_signature", e.to_string()))
}

/// Checks `body` against `sig_b64` (the `.sig` file written by `tauri signer
/// sign`) and the updater public key from `tauri.conf.json`, then parses it.
pub fn verify_policy(pubkey_b64: &str, body: &[u8], sig_b64: &str) -> Result<Policy, AppError> {
    let key = minisign_verify::PublicKey::decode(&base64_text(pubkey_b64)?)
        .map_err(|e| AppError::new("update_signature", e.to_string()))?;
    let sig = minisign_verify::Signature::decode(&base64_text(sig_b64)?)
        .map_err(|e| AppError::new("update_signature", e.to_string()))?;
    key.verify(body, &sig, false)
        .map_err(|_| AppError::new("update_signature", "policy.json signature does not match"))?;
    let policy: Policy =
        serde_json::from_slice(body).map_err(|e| AppError::new("update_policy", e.to_string()))?;
    let (version, minimum) = (parse(&policy.version)?, parse(&policy.minimum)?);
    if minimum > version {
        return Err(AppError::new(
            "update_policy",
            "minimum is newer than the release",
        ));
    }
    Ok(policy)
}

fn parse(v: &str) -> Result<Version, AppError> {
    Version::parse(v.trim().trim_start_matches('v'))
        .map_err(|e| AppError::new("update_policy", format!("{v}: {e}")))
}

/// Keeps the newest version and the highest minimum of two verified policies.
pub fn merge(cached: Option<Policy>, fresh: Option<Policy>) -> Option<Policy> {
    match (cached, fresh) {
        (None, p) | (p, None) => p,
        (Some(a), Some(b)) => {
            let newer = |x: &str, y: &str| parse(x).ok() >= parse(y).ok();
            let (version, notes, pub_date) = if newer(&b.version, &a.version) {
                (b.version, b.notes, b.pub_date)
            } else {
                (a.version, a.notes, a.pub_date)
            };
            let minimum = if newer(&b.minimum, &a.minimum) {
                b.minimum
            } else {
                a.minimum
            };
            Some(Policy {
                version,
                minimum,
                notes,
                pub_date,
            })
        }
    }
}

pub fn decide(current: &str, known: Option<&Policy>, failure: Option<&str>) -> UpdateStatus {
    let current_s = current.to_string();
    let (Ok(cur), Some(policy)) = (parse(current), known) else {
        return match failure {
            Some(reason) => UpdateStatus::Unknown {
                current: current_s,
                reason: reason.to_string(),
            },
            None => UpdateStatus::UpToDate { current: current_s },
        };
    };
    let (Ok(version), Ok(minimum)) = (parse(&policy.version), parse(&policy.minimum)) else {
        return UpdateStatus::UpToDate { current: current_s };
    };
    if cur < minimum {
        UpdateStatus::Required {
            current: current_s,
            version: policy.version.clone(),
            minimum: policy.minimum.clone(),
            notes: policy.notes.clone(),
        }
    } else if cur < version {
        UpdateStatus::Optional {
            current: current_s,
            version: policy.version.clone(),
            notes: policy.notes.clone(),
        }
    } else {
        UpdateStatus::UpToDate { current: current_s }
    }
}

pub fn cache_path(config_dir: &Path) -> PathBuf {
    config_dir.join("update-policy.json")
}

/// The cache holds only policies that were verified when fetched.
pub fn load_cache(config_dir: &Path) -> Option<Policy> {
    let text = std::fs::read_to_string(cache_path(config_dir)).ok()?;
    let policy: Policy = serde_json::from_str(&text).ok()?;
    (parse(&policy.version).is_ok() && parse(&policy.minimum).is_ok()).then_some(policy)
}

pub fn save_cache(config_dir: &Path, policy: &Policy) -> Result<(), AppError> {
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(cache_path(config_dir), serde_json::to_vec_pretty(policy)?)?;
    Ok(())
}

async fn get(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("{url}: HTTP {}", resp.status().as_u16()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_POLICY_BYTES {
        return Err(format!("{url}: too large"));
    }
    Ok(bytes.to_vec())
}

/// Fetches and verifies the policy from the first base that has a valid one.
pub async fn fetch_policy(pubkey_b64: &str, bases: &[String]) -> Result<Policy, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("JmdEditor/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    for base in bases {
        let body = get(&client, &format!("{base}/policy.json")).await;
        let sig = get(&client, &format!("{base}/policy.json.sig")).await;
        match (body, sig) {
            (Ok(body), Ok(sig)) => {
                match verify_policy(pubkey_b64, &body, &String::from_utf8_lossy(&sig)) {
                    Ok(policy) => return Ok(policy),
                    Err(e) => errors.push(e.message),
                }
            }
            (Err(e), _) | (_, Err(e)) => errors.push(e),
        }
    }
    Err(errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(version: &str, minimum: &str) -> Policy {
        Policy {
            version: version.into(),
            minimum: minimum.into(),
            notes: format!("notes {version}"),
            pub_date: String::new(),
        }
    }

    #[test]
    fn decides_from_semver_not_strings() {
        let p = policy("0.10.0", "0.9.0");
        assert!(matches!(
            decide("0.2.0", Some(&p), None),
            UpdateStatus::Required { .. }
        ));
        assert!(matches!(
            decide("0.9.0", Some(&p), None),
            UpdateStatus::Optional { .. }
        ));
        assert!(matches!(
            decide("0.10.0", Some(&p), None),
            UpdateStatus::UpToDate { .. }
        ));
        assert!(matches!(
            decide("0.11.0", Some(&p), None),
            UpdateStatus::UpToDate { .. }
        ));
    }

    #[test]
    fn a_failed_check_never_forces() {
        assert!(matches!(
            decide("0.1.0", None, Some("offline")),
            UpdateStatus::Unknown { .. }
        ));
        // …but a requirement verified earlier still applies offline.
        let cached = policy("0.3.0", "0.2.0");
        assert!(matches!(
            decide("0.1.0", Some(&cached), Some("offline")),
            UpdateStatus::Required { .. }
        ));
    }

    #[test]
    fn a_stale_policy_cannot_lower_the_minimum() {
        let merged = merge(
            Some(policy("0.3.0", "0.3.0")),
            Some(policy("0.2.0", "0.1.0")),
        )
        .unwrap();
        assert_eq!(
            (merged.version.as_str(), merged.minimum.as_str()),
            ("0.3.0", "0.3.0")
        );
        let merged = merge(
            Some(policy("0.3.0", "0.1.0")),
            Some(policy("0.4.0", "0.4.0")),
        )
        .unwrap();
        assert_eq!(
            (merged.version.as_str(), merged.minimum.as_str()),
            ("0.4.0", "0.4.0")
        );
    }

    // Signed with a throwaway key: `tauri signer generate` + `tauri signer sign`.
    const TEST_PUBKEY: &str = include_str!("../tests/update/test.key.pub");
    const TEST_BODY: &[u8] = include_bytes!("../tests/update/policy.json");
    const TEST_SIG: &str = include_str!("../tests/update/policy.json.sig");

    #[test]
    fn verifies_a_signed_policy_and_rejects_tampering() {
        let p = verify_policy(TEST_PUBKEY, TEST_BODY, TEST_SIG).unwrap();
        assert_eq!(p.minimum, "0.2.0");
        let tampered = String::from_utf8(TEST_BODY.to_vec())
            .unwrap()
            .replace("0.2.0", "0.1.0");
        let err = verify_policy(TEST_PUBKEY, tampered.as_bytes(), TEST_SIG).unwrap_err();
        assert_eq!(err.code, "update_signature");
    }

    #[test]
    fn cache_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_cache(dir.path()), None);
        let p = policy("0.2.0", "0.2.0");
        save_cache(dir.path(), &p).unwrap();
        assert_eq!(load_cache(dir.path()), Some(p));
    }
}
