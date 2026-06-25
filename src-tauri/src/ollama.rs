use serde::Serialize;
use specta::Type;
use std::process::Command;
use tauri::{AppHandle, Emitter};

const OLLAMA_HOST: &str = "http://localhost:11434";

#[derive(Debug, Serialize, Type, Clone)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub models: Vec<String>,
    pub has_model: bool,
}

/// Resolve the ollama binary path across common install locations.
fn ollama_bin() -> Option<String> {
    if let Ok(out) = Command::new("which").arg("ollama").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(p);
            }
        }
    }
    for c in [
        "/opt/homebrew/bin/ollama",
        "/usr/local/bin/ollama",
        "/usr/bin/ollama",
    ] {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

fn ollama_version(bin: &str) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// GET /api/tags — also serves as the running/health check.
fn fetch_models() -> Option<Vec<String>> {
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
        .ok()?
        .get(format!("{}/api/tags", OLLAMA_HOST))
        .send()
        .ok()?
        .text()
        .ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let models = json
        .get("models")?
        .as_array()?
        .iter()
        .filter_map(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    Some(models)
}

fn status(app: &AppHandle) -> OllamaStatus {
    let bin = ollama_bin();
    let installed = bin.is_some();
    let version = bin.as_deref().and_then(ollama_version);
    let models = fetch_models();
    let running = models.is_some();
    let models = models.unwrap_or_default();
    let configured = crate::settings::get_settings(app).ollama_model;
    let has_model = models.iter().any(|m| {
        m == &configured
            || m.starts_with(&format!(
                "{}:",
                configured.split(':').next().unwrap_or(&configured)
            ))
    });
    OllamaStatus {
        installed,
        running,
        version,
        models,
        has_model,
    }
}

#[tauri::command]
#[specta::specta]
pub fn ollama_status(app: AppHandle) -> OllamaStatus {
    status(&app)
}

#[tauri::command]
#[specta::specta]
pub fn ollama_install(app: AppHandle) -> Result<(), String> {
    if ollama_bin().is_some() {
        return Ok(());
    }
    // macOS: prefer Homebrew if present. Otherwise instruct the UI.
    let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string());
    let brew = match brew {
        Some(b) => b,
        None => {
            return Err(
                "Homebrew not found. Please install Ollama from https://ollama.com/download"
                    .to_string(),
            )
        }
    };
    let _ = app.emit("ollama-log", "Installing Ollama via Homebrew…".to_string());
    let out = Command::new(brew)
        .args(["install", "ollama"])
        .output()
        .map_err(|e| format!("brew failed to launch: {}", e))?;
    let _ = app.emit(
        "ollama-log",
        String::from_utf8_lossy(&out.stdout).to_string(),
    );
    if !out.status.success() {
        return Err(format!(
            "brew install ollama failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn ollama_start(app: AppHandle) -> Result<OllamaStatus, String> {
    if fetch_models().is_some() {
        return Ok(status(&app));
    } // already running
    let bin = ollama_bin().ok_or("Ollama is not installed")?;
    // Spawn `ollama serve` detached; we don't hold the child (server is long-lived).
    Command::new(bin)
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start ollama serve: {}", e))?;
    // Poll health up to ~10s.
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if fetch_models().is_some() {
            let s = status(&app);
            let _ = app.emit("ollama-status", s.clone());
            return Ok(s);
        }
    }
    Err("Ollama server did not become ready in time".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn ollama_pull(app: AppHandle, model: String) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let bin = ollama_bin().ok_or("Ollama is not installed")?;
    let mut child = Command::new(bin)
        .args(["pull", &model])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start ollama pull: {}", e))?;
    // Ollama writes progress to stderr.
    if let Some(err) = child.stderr.take() {
        let reader = BufReader::new(err);
        for line in reader.lines().map_while(Result::ok) {
            // Lines look like "pulling manifest", "pulling abc... 45%". Extract a percent if present.
            let percent = line
                .split_whitespace()
                .find_map(|t| t.strip_suffix('%').and_then(|n| n.parse::<u32>().ok()));
            let _ = app.emit(
                "ollama-pull-progress",
                serde_json::json!({
                    "model": model, "status": line, "percent": percent
                }),
            );
        }
    }
    let st = child
        .wait()
        .map_err(|e| format!("pull wait failed: {}", e))?;
    if !st.success() {
        return Err(format!("ollama pull {} failed", model));
    }
    let _ = app.emit(
        "ollama-pull-progress",
        serde_json::json!({
            "model": model, "status": "complete", "percent": 100
        }),
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn ollama_ensure_ready(app: AppHandle) -> Result<OllamaStatus, String> {
    if ollama_bin().is_none() {
        ollama_install(app.clone())?;
    }
    if fetch_models().is_none() {
        ollama_start(app.clone())?;
    }
    let model = crate::settings::get_settings(&app).ollama_model;
    let s = status(&app);
    if !s.has_model {
        ollama_pull(app.clone(), model)?;
    }
    Ok(status(&app))
}

/// Best-effort background auto-start used at app launch.
pub fn auto_start_if_enabled(app: &AppHandle) {
    let settings = crate::settings::get_settings(app);
    if !settings.ollama_auto_start {
        return;
    }
    if ollama_bin().is_none() {
        return;
    }
    if fetch_models().is_some() {
        return;
    }
    let app2 = app.clone();
    std::thread::spawn(move || {
        let _ = ollama_start(app2);
    });
}
