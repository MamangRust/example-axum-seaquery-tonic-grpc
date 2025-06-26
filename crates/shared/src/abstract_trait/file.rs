use std::sync::Arc;

use async_trait::async_trait;
use axum::{Json, http::StatusCode};

use crate::domain::{DeleteResponse, UploadResponse};

pub type DynFileService = Arc<dyn FileServiceTrait + Send + Sync>;

#[async_trait]
pub trait FileServiceTrait {
    async fn upload_image(
        &self,
        upload_dir: &str,
        original_filename: String,
        content_type: String,
        file_data: Vec<u8>,
    ) -> Result<Json<UploadResponse>, (StatusCode, Json<UploadResponse>)>;
    async fn delete_image(
        &self,
        upload_dir: &str,
        file_name: &str,
    ) -> Result<Json<DeleteResponse>, (StatusCode, Json<DeleteResponse>)>;
}
