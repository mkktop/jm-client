use reqwest::{Client, StatusCode};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/mkktop/jm-client/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/mkktop/jm-client/releases/latest";

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStatus {
    Available,
    Latest,
    Unpublished,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub status: UpdateStatus,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    body: Option<String>,
    published_at: Option<String>,
}

fn release_version(tag: &str) -> Result<Version, String> {
    let tag = tag.trim();
    let version = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);
    Version::parse(version).map_err(|_| "发布版本号格式不正确，请前往项目发布页查看。".to_string())
}

fn update_result(current: &str, release: Option<Release>) -> Result<UpdateCheck, String> {
    let current_version = Version::parse(current)
        .map_err(|_| "无法识别当前客户端版本，请前往项目发布页查看。".to_string())?;
    let mut result = UpdateCheck {
        status: UpdateStatus::Unpublished,
        current_version: current.to_string(),
        latest_version: None,
        release_notes: None,
        published_at: None,
    };
    if let Some(release) = release.filter(|r| !r.draft && !r.prerelease) {
        let latest = release_version(&release.tag_name)?;
        if !latest.pre.is_empty() {
            return Ok(result);
        }
        result.status = if latest.cmp_precedence(&current_version).is_gt() {
            UpdateStatus::Available
        } else {
            UpdateStatus::Latest
        };
        result.latest_version = Some(latest.to_string());
        result.release_notes = release.body.filter(|notes| !notes.trim().is_empty());
        result.published_at = release.published_at;
    }
    Ok(result)
}

pub async fn check(current: &str, proxy: Option<&str>) -> Result<UpdateCheck, String> {
    // 更新请求使用独立的 HTTPS 客户端，不沿用漫画接口的证书与会话设置。
    let mut builder = Client::builder()
        .user_agent(format!("JM-Client/{current}"))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20));
    if let Some(proxy) = proxy.filter(|value| !value.trim().is_empty()) {
        builder = builder.proxy(
            reqwest::Proxy::all(proxy.trim()).map_err(|_| "代理地址无效，请检查网络连接设置。")?,
        );
    }
    let client = builder.build().map_err(|_| "无法初始化更新连接。")?;
    let response = client
        .get(LATEST_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                "检查更新超时，请检查网络或代理设置后重试。"
            } else {
                "无法连接更新服务器，请检查网络或代理设置后重试。"
            }
        })?;
    match response.status() {
        StatusCode::NOT_FOUND => return update_result(current, None),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
            return Err("更新服务暂时限制了请求，请稍后重试。".to_string());
        }
        status if !status.is_success() => {
            return Err(format!("更新服务暂时不可用（HTTP {status}），请稍后重试。"));
        }
        _ => {}
    }
    let release = response
        .json::<Release>()
        .await
        .map_err(|_| "无法读取更新信息，请稍后重试。")?;
    update_result(current, Some(release))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.to_string(),
            draft: false,
            prerelease: false,
            body: Some("修复与改进".to_string()),
            published_at: Some("2026-11-02T00:00:00Z".to_string()),
        }
    }

    #[test]
    fn compares_semver_instead_of_strings() {
        let result = update_result("0.9.0", Some(release("v0.10.0"))).unwrap();
        assert_eq!(result.status, UpdateStatus::Available);
        assert_eq!(result.latest_version.as_deref(), Some("0.10.0"));
        assert_eq!(result.release_notes.as_deref(), Some("修复与改进"));
    }

    #[test]
    fn does_not_offer_equal_or_older_versions() {
        for tag in ["0.1.0", "v0.0.9", "V0.1.0+build.2"] {
            assert_eq!(
                update_result("0.1.0+build.1", Some(release(tag)))
                    .unwrap()
                    .status,
                UpdateStatus::Latest
            );
        }
    }

    #[test]
    fn stable_release_upgrades_a_prerelease_client() {
        assert_eq!(
            update_result("0.2.0-beta.1", Some(release("v0.2.0")))
                .unwrap()
                .status,
            UpdateStatus::Available
        );
    }

    #[test]
    fn unpublished_is_distinct_from_up_to_date() {
        let result = update_result("0.1.0", None).unwrap();
        assert_eq!(result.status, UpdateStatus::Unpublished);
        assert!(result.latest_version.is_none());
    }

    #[test]
    fn ignores_drafts_and_prereleases() {
        let mut draft = release("v0.2.0");
        draft.draft = true;
        let mut preview = release("v0.2.0-beta.1");
        preview.prerelease = true;
        for release in [draft, preview, release("v0.2.0-rc.1")] {
            assert_eq!(
                update_result("0.1.0", Some(release)).unwrap().status,
                UpdateStatus::Unpublished
            );
        }
    }

    #[test]
    fn malformed_versions_are_reported_as_errors() {
        assert!(update_result("0.1.0", Some(release("nightly"))).is_err());
        assert!(update_result("unknown", None).is_err());
    }
}
