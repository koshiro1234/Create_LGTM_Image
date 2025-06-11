use crate::application::error::ApplicationError; // Added for handler return types
use axum::{
    body::Body,
    extract::{Multipart, Query, Json, State},
    http::{self, StatusCode, header::HeaderName},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;
// TokioFile, AsyncWriteExt, Cursor は upload_image_handler で一時ファイル保存が残るなら必要
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
// use std::io::Cursor; // LgtmService がバイト列を直接扱うので、ハンドラでは不要になることが多い

use crate::application::lgtm_service::LgtmService;
// LocalFileStorage の use は preview/download が直接ファイルを読むなら必要

#[derive(Clone)]
pub struct AppState {
    pub lgtm_service: Arc<LgtmService>,
    // pub file_storage: Arc<LocalFileStorage>, // 必要に応じて
}

// FetchImageParams DTO はここ
#[derive(Deserialize, Debug)]
pub struct FetchImageParams {
    pub url: String,
    pub text: Option<String>,
    #[serde(rename = "textColor")]
    pub text_color: Option<String>,
    #[serde(rename = "textPosition")]
    pub text_position: Option<String>,
    #[serde(rename = "outputFormat")]
    pub output_format: Option<String>,
}

pub async fn upload_image_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApplicationError> { // Changed to ApplicationError
    // Simplified error handling for multipart processing for this step
    // Proper error mapping from multipart errors to ApplicationError would be more robust
    while let Some(field) = multipart.next_field().await.map_err(|e| ApplicationError::LgtmGenerationFailed(format!("Multipart error: {}", e)))? {
        let data = field.bytes().await.map_err(|e| ApplicationError::LgtmGenerationFailed(format!("Failed to read bytes from multipart field: {}", e)))?;

        let text = "LGTM".to_string(); // Default text
        let text_color_hex = "#FFFFFFFF".to_string(); // Default color
        let text_position_str = "center".to_string(); // Default position
        let output_format_str = "png".to_string(); // Default output format

        let (processed_image_data, _content_type) = state.lgtm_service.generate_lgtm_image(
            data.to_vec(),
            text,
            text_color_hex,
            text_position_str,
            output_format_str, // "png"
        ).await?; // Use `?` due to `From<ApplicationError>` for `InfrastructureError`

        // TODO: ファイル保存は FileStorage サービス経由にしたい
        // For now, map IO errors to ApplicationError::InfrastructureError manually or via a helper
        let mut file = TokioFile::create("output.png").await.map_err(|e| ApplicationError::InfrastructureError(super::error::InfrastructureError::IoError(e)))?;
        file.write_all(&processed_image_data).await.map_err(|e| ApplicationError::InfrastructureError(super::error::InfrastructureError::IoError(e)))?;
    }

    Ok("画像アップロード完了".to_string())
}

pub async fn preview_image_handler(
) -> Result<impl IntoResponse, ApplicationError> { // Changed to ApplicationError
    let image_path = "output.png";
    let image_data = tokio::fs::read(image_path).await.map_err(|e| ApplicationError::InfrastructureError(super::error::InfrastructureError::IoError(e)))?;

    Response::builder()
        .header("Content-Type", "image/png")
        .body(Body::from(image_data))
        .map_err(|e| ApplicationError::LgtmGenerationFailed(format!("Failed to build preview response: {}", e)))
}

pub async fn download_image_handler(
) -> Result<impl IntoResponse, ApplicationError> { // Changed to ApplicationError
    let image_path = "output.png";
    let image_data = tokio::fs::read(image_path)
        .await
        .map_err(|e| ApplicationError::InfrastructureError(super::error::InfrastructureError::IoError(e)))?;

    Response::builder()
        .header("Content-Type", "image/png")
        .header("Content-Disposition", "attachment; filename=\"downloaded_image.png\"")
        .body(Body::from(image_data))
        .map_err(|e| ApplicationError::LgtmGenerationFailed(format!("Failed to build download response: {}", e)))
}

pub async fn fetch_image_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<FetchImageParams>,
) -> Result<impl IntoResponse, ApplicationError> { // Changed to ApplicationError
    let actual_text = params.text.unwrap_or_else(|| "LGTM".to_string());
    let actual_color_hex = params.text_color.unwrap_or_else(|| "#FFFFFFFF".to_string());
    let actual_position_str = params.text_position.unwrap_or_else(|| "center".to_string());
    let desired_format_str = params.output_format.unwrap_or_else(|| "png".to_string());

    let (processed_image_data, content_type) = state.lgtm_service.generate_lgtm_image_from_url(
        params.url,
        actual_text,
        actual_color_hex,
        actual_position_str,
        desired_format_str,
    ).await?; // Use `?`

    Response::builder()
        .header("Content-Type", content_type)
        .body(Body::from(processed_image_data))
        .map_err(|e| ApplicationError::LgtmGenerationFailed(format!("Failed to build fetch response: {}", e)))
}
