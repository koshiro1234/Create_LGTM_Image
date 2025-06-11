use thiserror::Error;
use crate::domain::error::DomainError; // ImageProcessorの実装でDomainErrorを返すことがあるため

#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("Image processing failed: {0}")]
    ImageProcessingError(String),

    #[error("File storage error: {0}")]
    FileStorageError(String),

    #[error("External API call failed: {0}")]
    ExternalApiError(String),

    #[error("Data decoding failed: {0}")]
    DecodingError(String),

    #[error("Underlying image library error")]
    ImageLibError(#[from] image::ImageError), // image::ImageError をラップ

    #[error("Underlying I/O error")]
    IoError(#[from] std::io::Error), // std::io::Error をラップ

    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error), // reqwest::Error をラップ

    #[error("Base64 decode error")]
    Base64DecodeError(#[from] base64::DecodeError),

    // ドメインエラーをラップする場合 (ImageProcessorの実装がDomainErrorを返す場合など)
    // これは通常、ImageProcessorトレイトのメソッドがDomainErrorを返し、
    // その実装であるDefaultImageProcessorがそれをそのまま返すか、
    // InfrastructureErrorに変換する場合に使用します。
    // 今回の設計ではImageProcessorトレイトがInfrastructureErrorを返すようにしたので、
    // このDomainErrorWrapperは直接的には使われないかもしれません。
    // しかし、他のインフラコンポーネントがドメインサービスを呼び、それがDomainErrorを返す場合には有用です。
    #[error("Domain Error Wrapper: {0}")]
    DomainErrorWrapper(#[from] DomainError),
}
