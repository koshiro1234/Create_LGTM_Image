use axum::{
    extract::Multipart,
    response::{ IntoResponse, Response},
    body::Body,
    routing::{ get, post },
    Router,
    http::StatusCode,
};
use image::io::Reader as ImageReader;
use image::{ ImageFormat, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::fs;
use std::io::Cursor;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tower_http::cors::{ CorsLayer, Any };
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/", ServeDir::new("frontend/build"))
        .route("/upload", post(upload_image))
        .route("/preview", get(preview_image))
        .route("/download", get(download_image))
        .layer(CorsLayer::new().allow_origin(Any));

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

async fn download_image() -> Result<impl IntoResponse, (StatusCode, &'static str)> {
   // 画像のパス
   let image_path = "output.png";
    
   // 画像のバイナリデータを読み込んでレスポンスとして返す
   let image_data = std::fs::read(image_path)
       .map_err(|_| (StatusCode::NOT_FOUND, "Image not found"))?;
   
   // バイナリデータをレスポンスとして返す
   Ok(Response::builder()
       .header("Content-Type", "image/png")
       .header("Content-Disposition", "attachment; filename=\"downloaded_image.png\"")
       .body(Body::from(image_data))
       .unwrap())
}

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

fn add_text(img: &mut RgbaImage) {
    let font_data = include_bytes!("../../DejaVu_Sans/DejaVuSans-Bold.ttf");
    let font = Font::try_from_bytes(font_data).expect("フォントの読み込み失敗");
    let scale = Scale { x: 100.0, y: 100.0 };
    let x = img.width() / 4;
    let y = img.height() / 2;
    draw_text_mut(img, Rgba([255, 255, 255, 255]), x as i32, y as i32, scale, &font, "LGTM");
}
