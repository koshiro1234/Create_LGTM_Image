use std::sync::Arc;
// use anyhow::Result; // Remove if fully transitioned
use super::error::ApplicationError; // Changed from anyhow::Result
use image::ImageFormat as InnerImageFormat;

use crate::domain::image_processor_trait::ImageProcessor;
use crate::domain::text_overlay::TextOverlay;
use crate::domain::color::Color as DomainColor;
use crate::domain::position::Position as DomainPosition;
// external_image_fetcherもインフラ層なので、トレイト経由でDIするのが望ましいが、今回は直接使う
use crate::infrastructure::external_image_fetcher::DefaultExternalImageFetcher;


pub struct LgtmService {
    image_processor: Arc<dyn ImageProcessor + Send + Sync>, // トレイトオブジェクトとして保持
    // external_image_fetcher: Arc<dyn ExternalImageFetcherTrait + Send + Sync>, // 本来はこうしたい
}

impl LgtmService {
    pub fn new(image_processor: Arc<dyn ImageProcessor + Send + Sync>) -> Self {
        Self { image_processor }
    }

    fn map_position_str_to_domain(&self, position_str: &str) -> DomainPosition {
        match position_str.to_lowercase().as_str() {
            "top-left" => DomainPosition::TopLeft,
            "top-center" => DomainPosition::TopCenter,
            "top-right" => DomainPosition::TopRight,
            "center-left" => DomainPosition::CenterLeft,
            "center" => DomainPosition::Center,
            "center-right" => DomainPosition::CenterRight,
            "bottom-left" => DomainPosition::BottomLeft,
            "bottom-center" => DomainPosition::BottomCenter,
            "bottom-right" => DomainPosition::BottomRight,
            _ => DomainPosition::Center, // Default
        }
    }

    fn map_format_str_to_enum(&self, format_str: &str) -> (InnerImageFormat, &'static str) {
        match format_str.to_lowercase().as_str() {
            "jpeg" | "jpg" => (InnerImageFormat::Jpeg, "image/jpeg"),
            "png" | _ => (InnerImageFormat::Png, "image/png"), // Default to PNG
        }
    }

    pub async fn generate_lgtm_image(
        &self,
        image_data: Vec<u8>,
        text: String,
        text_color_hex: String,
        text_position_str: String,
        output_format_str: String, // 出力フォーマット指定を追加
    ) -> Result<(Vec<u8>, &'static str), ApplicationError> { // Changed to ApplicationError
        println!("LgtmService: generate_lgtm_image called with format: {}", output_format_str);

        let color = self.image_processor.parse_hex_color(&text_color_hex);
        let position = self.map_position_str_to_domain(&text_position_str);

        let text_overlay = TextOverlay {
            text,
            color,
            position,
        };

        let (output_format_enum, content_type) = self.map_format_str_to_enum(&output_format_str);

        let processed_image_bytes = self.image_processor.add_text_to_image(
            image_data,
            None, // image_data からフォーマットを推測させる
            &text_overlay,
            output_format_enum,
        )?;

        Ok((processed_image_bytes, content_type))
    }

    pub async fn generate_lgtm_image_from_url(
        &self,
        image_url: String,
        text: String,
        text_color_hex: String,
        text_position_str: String,
        output_format_str: String,
    ) -> Result<(Vec<u8>, &'static str), ApplicationError> { // Changed to ApplicationError
        println!("LgtmService: generate_lgtm_image_from_url called for URL: {}", image_url);

        // インフラ層の具体的な fetcher を直接利用 (DIするのが望ましい)
        let image_fetcher = DefaultExternalImageFetcher::new();
        let image_data = image_fetcher.fetch_image_from_url_impl(&image_url).await?;

        self.generate_lgtm_image(
            image_data,
            text,
            text_color_hex,
            text_position_str,
            output_format_str
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::image_processor_trait::ImageProcessor;
    use crate::infrastructure::error::InfrastructureError; // ImageProcessorモックが返すエラー用
    use crate::domain::text_overlay::TextOverlay as DomainTextOverlayFull; // Renamed to avoid conflict
    use image::ImageFormat as InnerImageFormat; // モック内で使うため
    use std::sync::{Arc, Mutex};

    // 手動モック: ImageProcessor トレイトのテスト用実装
    #[derive(Clone)]
    struct MockImageProcessor {
        add_text_result: Arc<Mutex<Result<Vec<u8>, String>>>, // Error type is String for easier mocking
        parse_color_result: Arc<Mutex<DomainColor>>,
        add_text_called: Arc<Mutex<bool>>,
        last_text_overlay: Arc<Mutex<Option<DomainTextOverlayFull>>>
    }

    impl ImageProcessor for MockImageProcessor {
        fn add_text_to_image(
            &self,
            _image_bytes: Vec<u8>,
            _input_format_opt: Option<InnerImageFormat>,
            text_overlay: &DomainTextOverlayFull,
            _output_format: InnerImageFormat,
        ) -> Result<Vec<u8>, InfrastructureError> {
            let mut called_flag = self.add_text_called.lock().unwrap();
            *called_flag = true;
            let mut last_overlay_lock = self.last_text_overlay.lock().unwrap();
            *last_overlay_lock = Some(text_overlay.clone());

            self.add_text_result.lock().unwrap().as_ref()
                .map(|v| v.clone())
                .map_err(|s| InfrastructureError::ImageProcessingError(s.clone()))
        }

        fn parse_hex_color(&self, _hex_str: &str) -> DomainColor {
            self.parse_color_result.lock().unwrap().clone()
        }
    }

    #[tokio::test]
    async fn test_generate_lgtm_image_success() {
        let mock_image_processor = Arc::new(MockImageProcessor {
            add_text_result: Arc::new(Mutex::new(Ok(vec![1, 2, 3]))),
            parse_color_result: Arc::new(Mutex::new(DomainColor::new(0,0,0,255))),
            add_text_called: Arc::new(Mutex::new(false)),
            last_text_overlay: Arc::new(Mutex::new(None)),
        });

        let service = LgtmService::new(mock_image_processor.clone());

        let image_data = vec![4, 5, 6];
        let result = service.generate_lgtm_image(
            image_data,
            "Test".to_string(),
            "#000000".to_string(),
            "center".to_string(),
            "png".to_string()
        ).await;

        assert!(result.is_ok());
        let (data, content_type) = result.unwrap();
        assert_eq!(data, vec![1, 2, 3]);
        assert_eq!(content_type, "image/png");
        assert!(*mock_image_processor.add_text_called.lock().unwrap());

        let overlay_used = mock_image_processor.last_text_overlay.lock().unwrap();
        assert!(overlay_used.is_some());
        assert_eq!(overlay_used.as_ref().unwrap().text, "Test");
    }

    #[tokio::test]
    async fn test_generate_lgtm_image_processor_fails() {
        let mock_image_processor = Arc::new(MockImageProcessor {
             add_text_result: Arc::new(Mutex::new(Err("mock processing error".to_string()))),
             parse_color_result: Arc::new(Mutex::new(DomainColor::new(0,0,0,255))),
             add_text_called: Arc::new(Mutex::new(false)),
             last_text_overlay: Arc::new(Mutex::new(None)),
        });
        let service = LgtmService::new(mock_image_processor);

        let image_data = vec![4, 5, 6];
        let result = service.generate_lgtm_image(
            image_data,
            "Test".to_string(),
            "#000000".to_string(),
            "center".to_string(),
            "png".to_string()
        ).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            ApplicationError::InfrastructureError(infra_err) => {
                match infra_err {
                    InfrastructureError::ImageProcessingError(msg) => {
                        assert_eq!(msg, "mock processing error");
                    }
                    _ => panic!("Expected InfrastructureError::ImageProcessingError variant, got {:?}", infra_err),
                }
            }
            e => panic!("Expected ApplicationError::InfrastructureError, got {:?}", e),
        }
    }
}
