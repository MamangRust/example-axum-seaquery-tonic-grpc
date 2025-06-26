use async_trait::async_trait;
use axum::{Json, http::StatusCode};
use chrono::Local;
use std::{collections::HashMap, fs, path::Path};
use tokio::{fs::File, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    abstract_trait::FileServiceTrait,
    domain::{DeleteResponse, UploadResponse},
};

#[derive(Default)]
pub struct FileService {}

impl FileService {
    fn get_extension_from_mime(&self, mime_type: &str) -> Option<&'static str> {
        let mime_map: HashMap<&str, &str> =
            HashMap::from([("image/jpeg", "jpg"), ("image/png", "png")]);
        mime_map.get(mime_type).copied()
    }
}

#[async_trait]
impl FileServiceTrait for FileService {
    async fn upload_image(
        &self,
        upload_dir: &str,
        original_filename: String,
        content_type: String,
        file_data: Vec<u8>,
    ) -> Result<Json<UploadResponse>, (StatusCode, Json<UploadResponse>)> {
        if file_data.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(UploadResponse {
                    message: "File is empty or invalid".to_string(),
                    file_name: "".to_string(),
                    file_path: "".to_string(),
                }),
            ));
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let unique_id = Uuid::new_v4();
        let mut saved_file_name = format!("{}", unique_id);

        if let Some(ext) = Path::new(&original_filename).extension() {
            saved_file_name.push('.');
            saved_file_name.push_str(ext.to_str().unwrap());
        } else if let Some(ext) = self.get_extension_from_mime(&content_type) {
            saved_file_name.push('.');
            saved_file_name.push_str(ext);
        }

        let folder_path = Path::new(upload_dir).join(today);
        if !folder_path.exists() {
            fs::create_dir_all(&folder_path).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(UploadResponse {
                        message: "Failed to create upload directory".to_string(),
                        file_name: "".to_string(),
                        file_path: "".to_string(),
                    }),
                )
            })?;
        }

        let file_path = folder_path.join(&saved_file_name);

        let mut file = File::create(&file_path).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UploadResponse {
                    message: "Failed to save file".to_string(),
                    file_name: "".to_string(),
                    file_path: "".to_string(),
                }),
            )
        })?;

        file.write(&file_data).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UploadResponse {
                    message: "Failed to write file".to_string(),
                    file_name: "".to_string(),
                    file_path: "".to_string(),
                }),
            )
        })?;

        Ok(Json(UploadResponse {
            message: "File uploaded successfully".to_string(),
            file_name: saved_file_name,
            file_path: file_path.to_string_lossy().to_string(),
        }))
    }

    async fn delete_image(
        &self,
        upload_dir: &str,
        file_name: &str,
    ) -> Result<Json<DeleteResponse>, (StatusCode, Json<DeleteResponse>)> {
        let file_path = Path::new(upload_dir).join(file_name);

        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await.map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DeleteResponse {
                        message: "Failed to delete file".to_string(),
                    }),
                )
            })?;
            Ok(Json(DeleteResponse {
                message: "File deleted successfully".to_string(),
            }))
        } else {
            Err((
                StatusCode::NOT_FOUND,
                Json(DeleteResponse {
                    message: "File not found".to_string(),
                }),
            ))
        }
    }
}
