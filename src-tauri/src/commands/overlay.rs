use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn dismiss_overlay_result(app: AppHandle) {
    crate::overlay::reset_overlay_size(&app);
    crate::utils::hide_recording_overlay(&app);
}
