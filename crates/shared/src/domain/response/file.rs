use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct UploadResponse {
    pub message: String,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub message: String,
}
