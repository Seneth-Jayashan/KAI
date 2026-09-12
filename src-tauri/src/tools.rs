use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Clone)]
pub struct PermissionRequest {
    pub tool: String,
    pub description: String,
    pub level: String, // "green", "yellow", "red"
}

pub async fn ask_permission(app: &AppHandle, request: PermissionRequest) -> bool {
    // For V1, we'll emit an event and wait for a frontend response.
    // To simplify for now, if it's green we just allow.
    if request.level == "green" {
        return true;
    }

    // Send event to frontend to show a modal
    let _ = app.emit("permission-request", request.clone());

    // In a real implementation we would wait on a channel for the user's response.
    // For scaffolding, we will just return false on yellow/red until the async channel is set up.
    false
}

#[tauri::command]
pub async fn get_system_info(app: tauri::AppHandle) -> Result<String, String> {
    let req = PermissionRequest {
        tool: "get_system_info".to_string(),
        description: "Read system information (CPU, OS)".to_string(),
        level: "green".to_string(),
    };

    if ask_permission(&app, req).await {
        Ok(format!(
            "OS: {}, Arch: {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    } else {
        Err("Permission denied".to_string())
    }
}
