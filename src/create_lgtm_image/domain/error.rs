use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    // 必要に応じてドメイン固有のエラーを追加
    // 例: #[error("Color parsing failed: {0}")]
    //     ColorParseError(String),
}
