use image::ImageFormat; // Assuming ImageFormat will be used from the image crate

pub struct Image {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

impl Image {
    pub fn new(data: Vec<u8>, width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            data,
            width,
            height,
            format,
        }
    }
}
