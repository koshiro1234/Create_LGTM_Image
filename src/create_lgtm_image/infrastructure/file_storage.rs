use super::error::InfrastructureError; // Changed from anyhow::Result
// use anyhow::Result; // Remove if fully transitioned
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::fs; // fs::read を使うために追加

// ドメイン層で定義する FileStorage トレイトの具体的な実装
/*
pub trait FileStorage {
    async fn save_image(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn read_image(&self, path: &str) -> Result<Vec<u8>>;
}
*/

pub struct LocalFileStorage;

impl LocalFileStorage {
    pub fn new() -> Self {
        Self
    }

    pub async fn save_image_impl(&self, path: &str, data: &[u8]) -> Result<(), InfrastructureError> { // Changed to InfrastructureError
        let mut file = File::create(path).await.map_err(InfrastructureError::IoError)?;
        file.write_all(data).await.map_err(InfrastructureError::IoError)?;
        Ok(())
    }

    pub async fn read_image_impl(&self, path: &str) -> Result<Vec<u8>, InfrastructureError> { // Changed to InfrastructureError
        let data = fs::read(path).await.map_err(InfrastructureError::IoError)?;
        Ok(data)
    }
}
