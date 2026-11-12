use anyhow::{Context, Result, bail};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const LINKEDIN_API_BASE: &str = "https://api.linkedin.com";

pub struct LinkedInClient {
    http: Client,
    access_token: String,
    api_version: String,
}

#[derive(Debug, Serialize)]
pub struct InitializeUploadPayload<'a> {
    #[serde(rename = "initializeUploadRequest")]
    pub initialize_upload_request: InitializeUploadOwner<'a>,
}

#[derive(Debug, Serialize)]
pub struct InitializeUploadOwner<'a> {
    pub owner: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct InitializeUploadResponse {
    pub value: InitializeUploadValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeUploadValue {
    pub upload_url: String,
    pub image: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostRequest {
    pub author: String,
    pub commentary: String,
    pub visibility: String,
    pub distribution: PostDistribution,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<PostContent>,
    pub lifecycle_state: String,
    pub is_reshare_disabled_by_author: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PostDistribution {
    pub feed_distribution: String,
    pub target_entities: Vec<serde_json::Value>,
    pub third_party_distribution_channels: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PostContent {
    pub media: MediaContent,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaContent {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

impl LinkedInClient {
    pub fn new(access_token: String, api_version: String) -> Result<Self> {
        let http = Client::builder()
            .build()
            .context("Failed to initialize HTTP client")?;
        Ok(Self {
            http,
            access_token,
            api_version,
        })
    }

    fn rest_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token.trim()))
                .context("Invalid characters in access_token")?,
        );
        headers.insert(
            "LinkedIn-Version",
            HeaderValue::from_str(self.api_version.trim())
                .context("Invalid characters in api_version")?,
        );
        headers.insert(
            "X-Restli-Protocol-Version",
            HeaderValue::from_static("2.0.0"),
        );
        Ok(headers)
    }

    /// Initializes an image upload and uploads the binary image file to LinkedIn.
    /// Returns the resulting image URN (e.g., `urn:li:image:...`).
    pub async fn upload_image(&self, owner_urn: &str, image_path: &Path) -> Result<String> {
        let image_bytes = fs::read(image_path).with_context(|| {
            format!("Failed to read image file at '{}'", image_path.display())
        })?;

        let init_url = format!("{}/rest/images?action=initializeUpload", LINKEDIN_API_BASE);
        let init_payload = InitializeUploadPayload {
            initialize_upload_request: InitializeUploadOwner { owner: owner_urn },
        };

        let init_resp = self
            .http
            .post(&init_url)
            .headers(self.rest_headers()?)
            .json(&init_payload)
            .send()
            .await
            .context("Failed to send image initializeUpload request to LinkedIn")?;

        if !init_resp.status().is_success() {
            let status = init_resp.status();
            let err_body = init_resp.text().await.unwrap_or_default();
            bail!(
                "LinkedIn image initializeUpload failed (HTTP {}): {}",
                status,
                err_body
            );
        }

        let init_data: InitializeUploadResponse = init_resp
            .json()
            .await
            .context("Failed to parse LinkedIn initializeUpload response JSON")?;

        let upload_resp = self
            .http
            .put(&init_data.value.upload_url)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.access_token.trim()),
            )
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(image_bytes)
            .send()
            .await
            .context("Failed to upload image bytes to LinkedIn uploadUrl")?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status();
            let err_body = upload_resp.text().await.unwrap_or_default();
            bail!("LinkedIn binary image upload failed (HTTP {}): {}", status, err_body);
        }

        Ok(init_data.value.image)
    }

    /// Publishes a post to `/rest/posts` and returns the created post URN.
    pub async fn create_post(&self, payload: &CreatePostRequest) -> Result<String> {
        let url = format!("{}/rest/posts", LINKEDIN_API_BASE);
        let resp = self
            .http
            .post(&url)
            .headers(self.rest_headers()?)
            .json(payload)
            .send()
            .await
            .context("Failed to send create post request to LinkedIn")?;

        let status = resp.status();
        let post_id = resp
            .headers()
            .get("x-restli-id")
            .or_else(|| resp.headers().get("X-RestLi-Id"))
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            bail!("LinkedIn post creation failed (HTTP {}): {}", status, err_body);
        }

        Ok(post_id.unwrap_or_else(|| "(post created, no x-restli-id header returned)".to_string()))
    }

    /// Fetches `/v2/userinfo` using the configured OAuth2 access token.
    pub async fn get_userinfo(&self) -> Result<UserInfo> {
        let url = format!("{}/v2/userinfo", LINKEDIN_API_BASE);
        let resp = self
            .http
            .get(&url)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.access_token.trim()),
            )
            .send()
            .await
            .context("Failed to call LinkedIn /v2/userinfo")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            bail!(
                "LinkedIn /v2/userinfo request failed (HTTP {}): {}",
                status,
                err_body
            );
        }

        resp.json::<UserInfo>()
            .await
            .context("Failed to parse /v2/userinfo JSON response")
    }
}

pub fn build_post_payload(
    author_urn: &str,
    commentary: &str,
    image_urn: Option<String>,
    alt_text: Option<String>,
    visibility: &str,
) -> CreatePostRequest {
    let content = image_urn.map(|id| PostContent {
        media: MediaContent { id, alt_text },
    });

    CreatePostRequest {
        author: author_urn.to_string(),
        commentary: commentary.to_string(),
        visibility: visibility.to_string(),
        distribution: PostDistribution {
            feed_distribution: "MAIN_FEED".to_string(),
            target_entities: Vec::new(),
            third_party_distribution_channels: Vec::new(),
        },
        content,
        lifecycle_state: "PUBLISHED".to_string(),
        is_reshare_disabled_by_author: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_text_only_post_payload() {
        let payload = build_post_payload(
            "urn:li:person:123",
            "Hello LinkedIn!",
            None,
            None,
            "PUBLIC",
        );
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["author"], "urn:li:person:123");
        assert_eq!(json["commentary"], "Hello LinkedIn!");
        assert_eq!(json["visibility"], "PUBLIC");
        assert!(json.get("content").is_none());
    }

    #[test]
    fn builds_image_post_payload() {
        let payload = build_post_payload(
            "urn:li:organization:456",
            "Release day!",
            Some("urn:li:image:C5522AQ".to_string()),
            Some("Release banner".to_string()),
            "PUBLIC",
        );
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["author"], "urn:li:organization:456");
        assert_eq!(json["content"]["media"]["id"], "urn:li:image:C5522AQ");
        assert_eq!(json["content"]["media"]["altText"], "Release banner");
    }
}
