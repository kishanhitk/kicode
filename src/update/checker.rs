use crate::config::{ReleaseChannel, VERSION};
use crate::error::Result;
use serde::Deserialize;

const GITHUB_API_URL: &str = "https://api.github.com/repos/kishanhitk/kicode/releases";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    prerelease: bool,
    html_url: String,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub is_prerelease: bool,
    pub release_url: String,
    pub channel: ReleaseChannel,
}

impl UpdateInfo {
    pub fn has_update(&self) -> bool {
        compare_versions(&self.current_version, &self.latest_version) == std::cmp::Ordering::Less
    }
}

pub struct UpdateChecker {
    client: reqwest::Client,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Check for updates based on the specified release channel
    pub async fn check(&self, channel: ReleaseChannel) -> Result<Option<UpdateInfo>> {
        let releases = self.fetch_releases().await?;

        let latest = match channel {
            ReleaseChannel::Stable => releases.into_iter().find(|r| !r.prerelease),
            ReleaseChannel::Beta => {
                // For beta, prefer prerelease, fall back to stable
                let prerelease = releases.iter().find(|r| r.prerelease);
                if prerelease.is_some() {
                    releases.into_iter().find(|r| r.prerelease)
                } else {
                    releases.into_iter().find(|r| !r.prerelease)
                }
            }
        };

        match latest {
            Some(release) => {
                let latest_version = release.tag_name.trim_start_matches('v').to_string();
                let info = UpdateInfo {
                    current_version: VERSION.to_string(),
                    latest_version,
                    is_prerelease: release.prerelease,
                    release_url: release.html_url,
                    channel,
                };

                if info.has_update() {
                    Ok(Some(info))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    async fn fetch_releases(&self) -> Result<Vec<GitHubRelease>> {
        let response = self
            .client
            .get(GITHUB_API_URL)
            .header("User-Agent", format!("kicode/{}", VERSION))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| crate::error::KicodeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::error::KicodeError::Network(format!(
                "GitHub API returned status: {}",
                response.status()
            )));
        }

        let releases: Vec<GitHubRelease> = response
            .json()
            .await
            .map_err(|e| crate::error::KicodeError::Network(e.to_string()))?;

        Ok(releases)
    }
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two semantic version strings
/// Returns Ordering::Less if v1 < v2, Equal if v1 == v2, Greater if v1 > v2
fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let parse_version = |v: &str| -> (Vec<u32>, Option<String>) {
        // Split on hyphen to separate version from prerelease tag
        let (version_part, prerelease) = match v.split_once('-') {
            Some((ver, pre)) => (ver, Some(pre.to_string())),
            None => (v, None),
        };

        let parts: Vec<u32> = version_part
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        (parts, prerelease)
    };

    let (parts1, pre1) = parse_version(v1);
    let (parts2, pre2) = parse_version(v2);

    // Compare numeric parts
    let max_len = parts1.len().max(parts2.len());
    for i in 0..max_len {
        let p1 = parts1.get(i).copied().unwrap_or(0);
        let p2 = parts2.get(i).copied().unwrap_or(0);
        match p1.cmp(&p2) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    // If numeric parts are equal, handle prerelease:
    // - No prerelease > prerelease (1.0.0 > 1.0.0-beta.1)
    // - Compare prerelease strings lexicographically
    match (&pre1, &pre2) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater, // stable > prerelease
        (Some(_), None) => std::cmp::Ordering::Less,    // prerelease < stable
        (Some(p1), Some(p2)) => compare_prerelease(p1, p2),
    }
}

/// Compare prerelease identifiers (e.g., "beta.1" vs "beta.2")
fn compare_prerelease(pre1: &str, pre2: &str) -> std::cmp::Ordering {
    let parts1: Vec<&str> = pre1.split('.').collect();
    let parts2: Vec<&str> = pre2.split('.').collect();

    for (p1, p2) in parts1.iter().zip(parts2.iter()) {
        // Try numeric comparison first
        match (p1.parse::<u32>(), p2.parse::<u32>()) {
            (Ok(n1), Ok(n2)) => match n1.cmp(&n2) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            },
            _ => match p1.cmp(p2) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            },
        }
    }

    parts1.len().cmp(&parts2.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions_basic() {
        assert_eq!(
            compare_versions("1.0.0", "1.0.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(compare_versions("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_versions("1.0.1", "1.0.0"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_versions_major_minor() {
        assert_eq!(compare_versions("1.0.0", "2.0.0"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_versions("1.5.0", "1.10.0"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_versions("2.0.0", "1.9.9"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_versions_prerelease() {
        // Stable is greater than prerelease
        assert_eq!(
            compare_versions("1.0.0", "1.0.0-beta.1"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.0.0-beta.1", "1.0.0"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_versions_prerelease_ordering() {
        assert_eq!(
            compare_versions("1.0.0-beta.1", "1.0.0-beta.2"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_versions("1.0.0-beta.2", "1.0.0-beta.1"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.0.0-alpha.1", "1.0.0-beta.1"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_versions_different_lengths() {
        assert_eq!(compare_versions("1.0", "1.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(compare_versions("1.0.0", "1.0"), std::cmp::Ordering::Equal);
        assert_eq!(compare_versions("1.0", "1.0.1"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_update_info_has_update() {
        let info = UpdateInfo {
            current_version: "0.1.0".to_string(),
            latest_version: "0.2.0".to_string(),
            is_prerelease: false,
            release_url: "https://example.com".to_string(),
            channel: ReleaseChannel::Stable,
        };
        assert!(info.has_update());

        let info_no_update = UpdateInfo {
            current_version: "0.2.0".to_string(),
            latest_version: "0.1.0".to_string(),
            is_prerelease: false,
            release_url: "https://example.com".to_string(),
            channel: ReleaseChannel::Stable,
        };
        assert!(!info_no_update.has_update());
    }
}
