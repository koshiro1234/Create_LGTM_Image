use crate::domain::image::Image as DomainImage;
use crate::domain::text_overlay::TextOverlay as DomainTextOverlay;
use crate::domain::color::Color as DomainColor; // 追加
use crate::domain::position::Position as DomainPosition; // 追加
use crate::infrastructure::error::InfrastructureError; // Changed from DomainError
// use anyhow::Result; // Removed as no longer directly used by trait methods
use image::ImageFormat as InnerImageFormat; // imageクレートのImageFormatをインポート

// このトレイトは、ドメインの型を受け取り、ドメインの型または結果を返す
pub trait ImageProcessor {
    fn add_text_to_image(
        &self,
        image_bytes: Vec<u8>,
        input_format_opt: Option<InnerImageFormat>,
        text_overlay: &DomainTextOverlay,
        output_format: InnerImageFormat,
    ) -> Result<Vec<u8>, InfrastructureError>; // Changed to InfrastructureError

    fn parse_hex_color(&self, hex_str: &str) -> DomainColor;
}
