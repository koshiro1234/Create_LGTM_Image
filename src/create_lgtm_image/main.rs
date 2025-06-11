// main.rs
pub mod domain;
pub mod application;
pub mod infrastructure;

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
    http::header::HeaderName, // axum::http::header::HeaderName に修正
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;

use infrastructure::axum_handler::{
    upload_image_handler,
    preview_image_handler,
    download_image_handler,
    fetch_image_handler,
    AppState,
};
use application::lgtm_service::LgtmService;
use infrastructure::image_processor::DefaultImageProcessor; // LgtmServiceに渡すために必要

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(vec![HeaderName::from_static("content-type")]);

    // ImageProcessor のインスタンスを作成
    let image_processor = Arc::new(DefaultImageProcessor::new());

    // LgtmService のインスタンスを作成し、ImageProcessor を注入
    let lgtm_service = Arc::new(LgtmService::new(image_processor));

    // AppState の初期化 (image_processor フィールドはもうない)
    let app_state = Arc::new(AppState {
        lgtm_service, // LgtmService のインスタンスを渡す
        // file_storage: Arc::new(LocalFileStorage::new()), // 必要なら
    });

    let app = Router::new()
        .nest_service("/", ServeDir::new("frontend/build"))
        .route("/upload", post(upload_image_handler))
        .route("/preview", get(preview_image_handler))
        .route("/download", get(download_image_handler))
        .route("/fetch", post(fetch_image_handler))
        .with_state(app_state)
        .layer(cors);

    println!("Server starting on 0.0.0.0:3300");
    axum::Server::bind(&"0.0.0.0:3300".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
