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
        add_text(&mut img, "LGTM", "#FFFFFFFF", "center");

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
    text: Option<String>,
    #[serde(rename = "textColor")]
    text_color: Option<String>,
    #[serde(rename = "textPosition")]
    text_position: Option<String>,
    #[serde(rename = "outputFormat")]
    output_format: Option<String>,
}

async fn fetch_image(Json(params): Json<FetchImageParams>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let url = &params.url;
    let actual_text = params.text.unwrap_or_else(|| "LGTM".to_string());
    let actual_color = params.text_color.unwrap_or_else(|| "#FFFFFFFF".to_string());
    let actual_position = params.text_position.unwrap_or_else(|| "center".to_string());
    let desired_format_str = params.output_format.unwrap_or_else(|| "png".to_string()).to_lowercase();

    let (image_format_enum, content_type_str) = match desired_format_str.as_str() {
        "jpeg" | "jpg" => (ImageFormat::Jpeg, "image/jpeg"),
        "png" | _ => (ImageFormat::Png, "image/png"), // Default to PNG
    };
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
    add_text(&mut img, &actual_text, &actual_color, &actual_position);

    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, image_format_enum).expect("画像の保存失敗");

    Ok(Response::builder()
        .header("Content-Type", content_type_str)
        .body(Body::from(buffer.into_inner()))
        .unwrap())
}

fn add_text(img: &mut RgbaImage, text: &str, color_hex: &str, position: &str) {
    let font_data = include_bytes!("../../DejaVu_Sans/DejaVuSans-Bold.ttf");
    let font = Font::try_from_bytes(font_data).expect("フォントの読み込み失敗");

    let color = parse_hex_color(color_hex);

    // Initial font scale based on image height.
    // Adjusted for very long text to prevent excessively small font from the start.
    let mut current_scale_val = if text.len() > 20 { // Heuristic for "very long"
        img.height() as f32 / (text.len() as f32 / 2.5) // More aggressive scaling down for long text
    } else if text.len() > 10 {
        img.height() as f32 / (text.len() as f32 / 1.8) // Moderate scaling for medium text
    } else {
        img.height() as f32 / 5.0 // Default for short text
    };
    // Ensure scale is not excessively small or zero if image height is tiny or text extremely long
    if current_scale_val < 1.0 { current_scale_val = 1.0; }

    let mut scale = Scale::uniform(current_scale_val);

    // Calculate text width and height with this initial scale
    let mut v_metrics = font.v_metrics(scale);
    let mut glyphs: Vec<_> = font.layout(text, scale, point(0.0, 0.0)).collect();
    let mut text_width = glyphs.iter().filter_map(|g| g.pixel_bounding_box()).map(|bb| bb.max.x as f32).last().unwrap_or(0.0);
    let mut text_height = v_metrics.ascent - v_metrics.descent; // Full height of the text block

    // Adjust scale if text width is too large for the image (e.g., > 90% of image width)
    let max_text_width_ratio = 0.90;
    if text_width > img.width() as f32 * max_text_width_ratio && text_width > 0.0 {
        let new_scale_factor = (img.width() as f32 * max_text_width_ratio) / text_width;
        current_scale_val *= new_scale_factor;
        if current_scale_val < 1.0 { current_scale_val = 1.0; } // Prevent scale from becoming too small
        scale = Scale::uniform(current_scale_val);

        // Recalculate metrics with the new, adjusted scale
        v_metrics = font.v_metrics(scale);
        glyphs = font.layout(text, scale, point(0.0, 0.0)).collect();
        text_width = glyphs.iter().filter_map(|g| g.pixel_bounding_box()).map(|bb| bb.max.x as f32).last().unwrap_or(0.0);
        text_height = v_metrics.ascent - v_metrics.descent;
    }

    let final_scale = scale;
    let v_metrics_final = v_metrics; // Use the potentially adjusted v_metrics
    let actual_text_glyph_height = text_height; // Height of the text glyphs based on final scale

    // Calculate base x and y for text drawing based on position string
    let (mut x_pos, mut y_pos_base) = match position {
        "top-left" => (0.0, 0.0),
        "top-center" => ((img.width() as f32 - text_width) / 2.0, 0.0),
        "top-right" => (img.width() as f32 - text_width, 0.0),
        "center-left" => (0.0, (img.height() as f32 - actual_text_glyph_height) / 2.0),
        "center" => ((img.width() as f32 - text_width) / 2.0, (img.height() as f32 - actual_text_glyph_height) / 2.0),
        "center-right" => (img.width() as f32 - text_width, (img.height() as f32 - actual_text_glyph_height) / 2.0),
        "bottom-left" => (0.0, img.height() as f32 - actual_text_glyph_height),
        "bottom-center" => ((img.width() as f32 - text_width) / 2.0, img.height() as f32 - actual_text_glyph_height),
        "bottom-right" => (img.width() as f32 - text_width, img.height() as f32 - actual_text_glyph_height),
        _ => ((img.width() as f32 - text_width) / 2.0, (img.height() as f32 - actual_text_glyph_height) / 2.0), // Default
    };

    // Ensure x_pos is not negative (can happen if text_width is slightly larger than image due to approximations)
    if x_pos < 0.0 { x_pos = 0.0; }
    if y_pos_base < 0.0 {y_pos_base = 0.0; }


    // Adjust y_pos to account for font ascent (baseline for drawing)
    let y_pos = y_pos_base + v_metrics_final.ascent;

    draw_text_mut(img, color, x_pos as i32, y_pos as i32, final_scale, &font, text);
}

// Helper function to parse hex color string
fn parse_hex_color(hex_str: &str) -> Rgba<u8> {
    let hex = hex_str.trim_start_matches('#');
    let default_color = Rgba([255, 255, 255, 255]); // White

    match hex.len() {
        6 => { // RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            Rgba([r, g, b, 255])
        }
        8 => { // RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            Rgba([r, g, b, a])
        }
        _ => default_color, // Invalid length
    }
}
