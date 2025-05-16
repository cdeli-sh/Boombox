use crate::audio;
use crate::filesystem;
use crate::sqlite;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use axum::body::Body;
use axum::http::{header, HeaderValue, Response as HttpResponse};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

// Shared state for the API
pub struct ApiState {
    boombox: Arc<Mutex<audio::Boombox>>,
}

// Request and response types
#[derive(Deserialize)]
struct FoldersRequest {
    folders: Vec<String>,
}

#[derive(Deserialize)]
struct FolderRequest {
    folder: String,
}

#[derive(Deserialize)]
struct PathRequest {
    path: String,
}

#[derive(Deserialize)]
struct VolumeRequest {
    volume: f32,
}

#[derive(Deserialize)]
struct DeviceRequest {
    device_name: String,
}

#[derive(Serialize)]
struct FoldersResponse {
    folders: Vec<String>,
}

#[derive(Serialize)]
struct FilesResponse {
    files: Vec<String>,
}

#[derive(Serialize)]
struct DevicesResponse {
    devices: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// Error handling
enum ApiError {
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            error: error_message,
        });

        (status, body).into_response()
    }
}

// Convert Result<T, String> to Result<T, ApiError>
fn map_err<T>(result: Result<T, String>) -> Result<T, ApiError> {
    result.map_err(ApiError::Internal)
}

// API routes
async fn add_folders(
    Json(payload): Json<FoldersRequest>,
) -> Result<StatusCode, ApiError> {
    map_err(sqlite::add_folders(payload.folders))?;
    Ok(StatusCode::OK)
}

async fn get_folders() -> Result<Json<FoldersResponse>, ApiError> {
    let folders = map_err(sqlite::get_folders())?
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    Ok(Json(FoldersResponse { folders }))
}

async fn get_files_in_folder(
    Json(payload): Json<FolderRequest>,
) -> Result<Json<FilesResponse>, ApiError> {
    let files = map_err(filesystem::get_files_in_path(payload.folder))?;
    Ok(Json(FilesResponse { files }))
}

async fn play_audio(
    State(state): State<ApiState>,
    Json(payload): Json<PathRequest>,
) -> Result<StatusCode, ApiError> {
    map_err(state.boombox.lock().unwrap().play_file(payload.path))?;
    Ok(StatusCode::OK)
}

async fn pause_audio(
    State(state): State<ApiState>,
) -> Result<StatusCode, ApiError> {
    map_err(state.boombox.lock().unwrap().pause())?;
    Ok(StatusCode::OK)
}

async fn resume_audio(
    State(state): State<ApiState>,
) -> Result<StatusCode, ApiError> {
    map_err(state.boombox.lock().unwrap().resume())?;
    Ok(StatusCode::OK)
}

async fn stop_audio(
    State(state): State<ApiState>,
) -> Result<StatusCode, ApiError> {
    map_err(state.boombox.lock().unwrap().stop())?;
    Ok(StatusCode::OK)
}

async fn set_volume(
    State(state): State<ApiState>,
    Json(payload): Json<VolumeRequest>,
) -> Result<StatusCode, ApiError> {
    if payload.volume < 0.0 || payload.volume > 1.0 {
        return Err(ApiError::Internal("Volume must be between 0 and 1".into()));
    }
    map_err(state.boombox.lock().unwrap().set_volume(payload.volume))?;
    Ok(StatusCode::OK)
}

async fn list_audio_devices(
    State(state): State<ApiState>,
) -> Result<Json<DevicesResponse>, ApiError> {
    let devices = map_err(state.boombox.lock().unwrap().list_devices())?;
    Ok(Json(DevicesResponse { devices }))
}

async fn set_audio_device(
    State(state): State<ApiState>,
    Json(payload): Json<DeviceRequest>,
) -> Result<StatusCode, ApiError> {
    map_err(state.boombox.lock().unwrap().set_device(payload.device_name))?;
    Ok(StatusCode::OK)
}

async fn serve_index_html() -> impl IntoResponse {
  let mut file = match File::open("src-tauri/src/static/index.html").await {
      Ok(file) => file,
      Err(_) => {
          return HttpResponse::builder()
              .status(StatusCode::NOT_FOUND)
              .body(Body::from("HTML file not found"))
              .unwrap()
      }
  };

  let mut contents = Vec::new();
  if let Err(_) = file.read_to_end(&mut contents).await {
      return HttpResponse::builder()
          .status(StatusCode::INTERNAL_SERVER_ERROR)
          .body(Body::from("Failed to read HTML file"))
          .unwrap();
  }

  HttpResponse::builder()
      .status(StatusCode::OK)
      .header(header::CONTENT_TYPE, HeaderValue::from_static("text/html"))
      .body(Body::from(contents))
      .unwrap()
}


// Start the API server
pub async fn start_api_server(boombox: audio::Boombox) -> Result<(), String> {
    let shared_state = Arc::new(Mutex::new(boombox));
    
    let api_state = ApiState {
        boombox: shared_state,
    };

    let app = Router::new()
        .route("/", get(serve_index_html))
        .route("/folders", post(add_folders))
        .route("/folders", get(get_folders))
        .route("/files", post(get_files_in_folder))
        .route("/play", post(play_audio))
        .route("/pause", post(pause_audio))
        .route("/resume", post(resume_audio))
        .route("/stop", post(stop_audio))
        .route("/volume", post(set_volume))
        .route("/devices", get(list_audio_devices))
        .route("/device", post(set_audio_device))
        .with_state(api_state);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .map_err(|e| format!("Failed to bind to address: {}", e))?;
    
    println!("API server listening on http://127.0.0.1:3000");
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}