use crate::auth::keyring::{delete_icloud, load_icloud, save_icloud, ICloudCredentials};
use crate::error::{AppError, AppResult};

const PRINCIPAL_URL: &str = "https://caldav.icloud.com/";

/// Saves the credentials and verifies they work by issuing a `PROPFIND` request
/// against the iCloud CalDAV principal endpoint.
pub async fn save_and_verify(apple_id: String, app_password: String) -> AppResult<()> {
    let creds = ICloudCredentials { apple_id, app_password };
    verify(&creds).await?;
    save_icloud(&creds)?;
    Ok(())
}

pub async fn verify(creds: &ICloudCredentials) -> AppResult<()> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal/>
  </d:prop>
</d:propfind>"#;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), PRINCIPAL_URL)
        .basic_auth(&creds.apple_id, Some(&creds.app_password))
        .header("Depth", "0")
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    if status.as_u16() == 207 || status.is_success() {
        Ok(())
    } else if status.as_u16() == 401 {
        Err(AppError::CalDav("iCloud authentication failed (401)".into()))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(AppError::CalDav(format!(
            "iCloud PROPFIND returned {status}: {text}"
        )))
    }
}

pub fn revoke() -> AppResult<()> {
    delete_icloud()
}

pub fn is_connected() -> AppResult<bool> {
    Ok(load_icloud()?.is_some())
}
