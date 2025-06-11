use super::error::InfrastructureError; // Changed from anyhow::Result
// use anyhow::Result; // Remove if fully transitioned
use reqwest;
use base64::decode; // main.rs から移動

// ドメイン層で定義する ExternalImageFetcher トレイトの具体的な実装
/*
pub trait ExternalImageFetcher {
    async fn fetch_image_from_url(&self, url: &str) -> Result<Vec<u8>>;
}
*/

pub struct DefaultExternalImageFetcher;

impl DefaultExternalImageFetcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_image_from_url_impl(&self, url: &str) -> Result<Vec<u8>, InfrastructureError> { // Changed to InfrastructureError
        if url.starts_with("data:") {
            let base64_data = url.split(',').nth(1).ok_or_else(|| InfrastructureError::DecodingError("Invalid data URL: missing comma".to_string()))?;
            Ok(decode(base64_data).map_err(InfrastructureError::Base64DecodeError)?)
        } else {
            let response = reqwest::get(url).await.map_err(InfrastructureError::ReqwestError)?;
            Ok(response.bytes().await.map_err(InfrastructureError::ReqwestError)?.to_vec())
        }
    }
}
