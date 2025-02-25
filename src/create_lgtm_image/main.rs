use axum::{
    body::Body, extract::{Multipart, Query}, http::{self, StatusCode}, response::{IntoResponse, Response}, routing::{get, post}, Json, Router
};
use image::io::Reader as ImageReader;
use image::{ImageFormat, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale, point};
use serde::Deserialize;
use std::fs;
use std::io::Cursor;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use reqwest;
use http::header::HeaderName;
use base64::decode;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(vec![HeaderName::from_static("content-type")]);

    let app = Router::new()
        .nest_service("/", ServeDir::new("frontend/build"))
        .route("/upload", post(upload_image))
        .route("/preview", get(preview_image))
        .route("/download", get(download_image))
        .route("/fetch", post(fetch_image))
        .layer(cors);

    // サーバーの開始
    axum::Server::bind(&"0.0.0.0:3300".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// 画像プレビュー用のエンドポイント
async fn preview_image() -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let image_path = "output.png"; // 作成した画像のパス
    let image_data = fs::read(image_path).map_err(|_| (StatusCode::NOT_FOUND, "Image not found"))?;
    
    Ok(Response::builder()
        .header("Content-Type", "image/png")
        .body(Body::from(image_data))
        .unwrap())
}

// 画像ダウンロード用のエンドポイント
async fn download_image() -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    // 画像のパス
    let image_path = "output.png";
    
    // 画像のバイナリデータを読み込んでレスポンスとして返す
    let image_data = fs::read(image_path)
        .map_err(|_| (StatusCode::NOT_FOUND, "Image not found"))?;
    
    // バイナリデータをレスポンスとして返す
    Ok(Response::builder()
        .header("Content-Type", "image/png")
        .header("Content-Disposition", "attachment; filename=\"downloaded_image.png\"")
        .body(Body::from(image_data))
        .unwrap())
}

// 画像アップロード用のエンドポイント
async fn upload_image(mut multipart: Multipart) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();

        let reader = ImageReader::new(Cursor::new(data)).with_guessed_format().expect("フォーマット判定失敗");
        println!("Decord format: {:?}", reader.format());
        let img = reader.decode().expect("画像のデコード失敗");

        let mut img = img.to_rgba8();
        add_text(&mut img);

        // 画像を適切なフォーマットで保存
        let mut file = File::create("output.png").await.unwrap();
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageFormat::Png).expect("画像の保存失敗");

        file.write_all(&buffer.into_inner()).await.unwrap();
    }

    "画像アップロード完了"
}

// 画像URLから画像を取得して表示するエンドポイント
#[derive(Deserialize)]
struct FetchImageParams {
    url: String,
}

async fn fetch_image(Json(params): Json<FetchImageParams>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let url = &params.url;
    let bytes = if url.starts_with("data:") {
        // dataスキームを解析
        let base64_data = url.split(',').nth(1).ok_or((StatusCode::BAD_REQUEST, "Invalid data URL"))?;
        decode(base64_data).map_err(|_| (StatusCode::BAD_REQUEST, "Failed to decode base64 data"))?
    } else {
        // 通常のURLから画像を取得
        let response = reqwest::get(url).await.map_err(|e| {
            println!("Failed to fetch image: {:?}", e); // エラーメッセージを詳細に出力
            (StatusCode::BAD_REQUEST, "Failed to fetch image")
        })?;
        response.bytes().await.map_err(|e| {
            println!("Failed to read image: {:?}", e); // エラーメッセージを詳細に出力
            (StatusCode::BAD_REQUEST, "Failed to read image")
        })?.to_vec()
    };

    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().expect("フォーマット判定失敗");
    let img = reader.decode().expect("画像のデコード失敗");

    let mut img = img.to_rgba8();
    add_text(&mut img);

    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png).expect("画像の保存失敗");

    Ok(Response::builder()
        .header("Content-Type", "image/png")
        .body(Body::from(buffer.into_inner()))
        .unwrap())
}

fn add_text(img: &mut RgbaImage) {
    let font_data = include_bytes!("../../DejaVu_Sans/DejaVuSans-Bold.ttf");
    let font = Font::try_from_bytes(font_data).expect("フォントの読み込み失敗");

    // 画像サイズに基づいてフォントサイズを計算
    let scale = Scale::uniform(img.width() as f32 / 5.0);

    // 文字の幅と高さを計算
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font.layout("LGTM", scale, point(0.0, v_metrics.ascent)).collect();
    let width = glyphs.iter().rev().filter_map(|g| g.pixel_bounding_box().map(|b| b.max.x as f32)).next().unwrap_or(0.0);
    let height = v_metrics.ascent - v_metrics.descent;

    // フォントサイズを調整して文字の幅が画像の幅に収まるようにする
    let scale = Scale::uniform(scale.x * (img.width() as f32 / width));

    // 文字の位置を画像の中央に設定
    let x = 0; // 左寄せ
    let y = (img.height() as f32 - height) / 2.0 + v_metrics.ascent;

    draw_text_mut(img, Rgba([255, 255, 255, 255]), x as i32, y as i32, scale, &font, "LGTM");
}
