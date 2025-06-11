use crate::domain::image_processor_trait::ImageProcessor;
use crate::domain::image::Image as DomainImage; // Keep if used, or remove
use crate::domain::text_overlay::TextOverlay as DomainTextOverlay;
use crate::domain::color::Color as DomainColor;
use crate::domain::position::Position as DomainPosition;
use super::error::InfrastructureError; // Changed from anyhow::Result
// use anyhow::Result; // Remove if fully transitioned
use crate::domain::error::DomainError; // Not directly used in signature, but good for context
use image::{Rgba, RgbaImage, ImageFormat as InnerImageFormat}; // imageクレートの型
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale, point};
use std::io::Cursor;

// ドメイン層で定義する ImageProcessor トレイトの具体的な実装
// (トレイトの定義はドメイン層で行うが、ここでは仮で Trait をコメントアウトで記述)
/*
pub trait ImageProcessor {
    fn add_text_to_image(
        &self,
        image: DomainImage,
        text_overlay: DomainTextOverlay,
    ) -> Result<DomainImage>;
}
*/

pub struct DefaultImageProcessor;

impl DefaultImageProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl ImageProcessor for DefaultImageProcessor {
    // main.rs の add_text と parse_hex_color をここに移植・統合する
    // 入力はドメインの型、出力もドメインの型とする
    fn add_text_to_image(
        &self,
        image_bytes: Vec<u8>, // 元の画像のバイト列
        input_format_opt: Option<InnerImageFormat>, // 元の画像のフォーマット (推測に任せる場合はNone)
        text_overlay: &DomainTextOverlay,
        output_format: InnerImageFormat,
    ) -> Result<Vec<u8>, InfrastructureError> { // Changed to InfrastructureError
        let reader = match input_format_opt {
            Some(format) => image::io::Reader::with_format(Cursor::new(image_bytes), format),
            None => image::io::Reader::new(Cursor::new(image_bytes)).with_guessed_format().map_err(InfrastructureError::IoError)?,
        };
        let mut img = reader.decode().map_err(InfrastructureError::ImageLibError)?.to_rgba8();

        let font_data = include_bytes!("../../../DejaVu_Sans/DejaVuSans-Bold.ttf");
        let font = Font::try_from_bytes(font_data).ok_or_else(|| InfrastructureError::ImageProcessingError("Failed to load font".to_string()))?;

        let color = Rgba([
            text_overlay.color.r,
            text_overlay.color.g,
            text_overlay.color.b,
            text_overlay.color.a,
        ]);

        // テキストのスケールと位置計算 (main.rs のロジックを適用)
        // この部分は main.rs の add_text 関数のロジックをほぼそのまま持ってくる
        // ただし、入力として DomainTextOverlay の position を使う
        let text = &text_overlay.text;
        let mut current_scale_val = if text.len() > 20 {
            img.height() as f32 / (text.len() as f32 / 2.5)
        } else if text.len() > 10 {
            img.height() as f32 / (text.len() as f32 / 1.8)
        } else {
            img.height() as f32 / 5.0
        };
        if current_scale_val < 1.0 { current_scale_val = 1.0; }
        let mut scale = Scale::uniform(current_scale_val);
        let mut v_metrics = font.v_metrics(scale);
        let mut glyphs: Vec<_> = font.layout(text, scale, point(0.0, 0.0)).collect();
        let mut text_width = glyphs.iter().filter_map(|g| g.pixel_bounding_box()).map(|bb| bb.max.x as f32).last().unwrap_or(0.0);
        let mut text_height = v_metrics.ascent - v_metrics.descent;

        let max_text_width_ratio = 0.90;
        if text_width > img.width() as f32 * max_text_width_ratio && text_width > 0.0 {
            let new_scale_factor = (img.width() as f32 * max_text_width_ratio) / text_width;
            current_scale_val *= new_scale_factor;
            if current_scale_val < 1.0 { current_scale_val = 1.0; }
            scale = Scale::uniform(current_scale_val);
            v_metrics = font.v_metrics(scale);
            glyphs = font.layout(text, scale, point(0.0, 0.0)).collect();
            text_width = glyphs.iter().filter_map(|g| g.pixel_bounding_box()).map(|bb| bb.max.x as f32).last().unwrap_or(0.0);
            text_height = v_metrics.ascent - v_metrics.descent;
        }

        let final_scale = scale;
        let v_metrics_final = v_metrics;
        let actual_text_glyph_height = text_height;

        let (mut x_pos, mut y_pos_base) = match text_overlay.position {
            DomainPosition::TopLeft => (0.0, 0.0),
            DomainPosition::TopCenter => ((img.width() as f32 - text_width) / 2.0, 0.0),
            DomainPosition::TopRight => (img.width() as f32 - text_width, 0.0),
            DomainPosition::CenterLeft => (0.0, (img.height() as f32 - actual_text_glyph_height) / 2.0),
            DomainPosition::Center | DomainPosition::Custom { .. } => ((img.width() as f32 - text_width) / 2.0, (img.height() as f32 - actual_text_glyph_height) / 2.0), // Customは一旦Centerと同じ扱い
            DomainPosition::CenterRight => (img.width() as f32 - text_width, (img.height() as f32 - actual_text_glyph_height) / 2.0),
            DomainPosition::BottomLeft => (0.0, img.height() as f32 - actual_text_glyph_height),
            DomainPosition::BottomCenter => ((img.width() as f32 - text_width) / 2.0, img.height() as f32 - actual_text_glyph_height),
            DomainPosition::BottomRight => (img.width() as f32 - text_width, img.height() as f32 - actual_text_glyph_height),
        };
        if x_pos < 0.0 { x_pos = 0.0; }
        if y_pos_base < 0.0 {y_pos_base = 0.0; }
        let y_pos = y_pos_base + v_metrics_final.ascent;

        draw_text_mut(&mut img, color, x_pos as i32, y_pos as i32, final_scale, &font, text);

        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, output_format).map_err(InfrastructureError::ImageLibError)?;
        Ok(buffer.into_inner())
    }

    // main.rs の parse_hex_color をここに移植
    fn parse_hex_color(&self, hex_str: &str) -> DomainColor {
        let hex = hex_str.trim_start_matches('#');
        let default_color = DomainColor::new(255, 255, 255, 255); // White

        match hex.len() {
            6 => { // RRGGBB
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
                DomainColor::new(r, g, b, 255)
            }
            8 => { // RRGGBBAA
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                DomainColor::new(r, g, b, a)
            }
            _ => default_color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::color::Color as DomainColor;
    use crate::domain::position::Position as DomainPosition;
    use crate::domain::text_overlay::TextOverlay;
    use image::ImageFormat; // image クレートの ImageFormat
    use crate::infrastructure::error::InfrastructureError; // For error matching

    // parse_hex_color のテスト
    #[test]
    fn test_parse_hex_color_valid_formats() {
        let processor = DefaultImageProcessor::new();
        assert_eq!(processor.parse_hex_color("#FF0000"), DomainColor::new(255, 0, 0, 255));
        assert_eq!(processor.parse_hex_color("00FF00"), DomainColor::new(0, 255, 0, 255));
        assert_eq!(processor.parse_hex_color("#0000FF80"), DomainColor::new(0, 0, 255, 128));
    }

    #[test]
    fn test_parse_hex_color_invalid_formats() {
        let processor = DefaultImageProcessor::new();
        let default_white = DomainColor::new(255, 255, 255, 255);
        assert_eq!(processor.parse_hex_color("invalid"), default_white);
        assert_eq!(processor.parse_hex_color("#123"), default_white);
        assert_eq!(processor.parse_hex_color(""), default_white);
    }

    // add_text_to_image の基本的なテスト (エラーにならないこと、バイト列が返ること)
    // このテストでは、実際に生成された画像の内容までは検証しない (それはより複雑なセットアップが必要)
    #[test]
    fn test_add_text_to_image_runs_without_error() {
        let processor = DefaultImageProcessor::new();
        // ダミーの画像データ (小さな透明なPNG)
        // 1x1の透明なPNGのBase64エンコードデータ
        let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
        let image_bytes = base64::decode(base64_image).unwrap();

        let text_overlay = TextOverlay {
            text: "Test".to_string(),
            color: DomainColor::new(255,0,0,255),
            position: DomainPosition::Center,
        };

        let result = processor.add_text_to_image(
            image_bytes,
            Some(ImageFormat::Png), // 入力フォーマットを指定
            &text_overlay,
            ImageFormat::Png // 出力フォーマットを指定
        );
        assert!(result.is_ok());
        if let Ok(output_bytes) = result {
            assert!(!output_bytes.is_empty());
        }
    }

    #[test]
    fn test_add_text_to_image_invalid_image_data() {
        let processor = DefaultImageProcessor::new();
        let invalid_image_bytes = vec![1, 2, 3, 4]; // 明らかに不正な画像データ

        let text_overlay = TextOverlay {
            text: "Test".to_string(),
            color: DomainColor::new(255,0,0,255),
            position: DomainPosition::Center,
        };

        let result = processor.add_text_to_image(
            invalid_image_bytes,
            None, // フォーマット推測させる
            &text_overlay,
            ImageFormat::Png
        );
        assert!(result.is_err());
        if let Err(e) = result {
            println!("Got expected error for invalid image data: {:?}", e);
            match e {
                InfrastructureError::ImageLibError(_) => {} // OK
                // It could also be IoError if the format guessing fails at that stage
                InfrastructureError::IoError(_) => {} // Also OK
                _ => panic!("Expected ImageLibError or IoError for invalid image data, got {:?}", e),
            }
        }
    }
}
