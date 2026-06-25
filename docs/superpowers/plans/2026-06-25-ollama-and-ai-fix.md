# Apple Intelligence build fix + zero-touch Ollama — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the lisper backend build on Command Line Tools (fix the Apple Intelligence Swift + add a stub flag), and make Ollama a zero-touch default local LLM (install/launch/pull/use from the app) with a setup UI.

**Architecture:** Tauri 2.x (Rust + React/TS). Workstream A fixes `swift/apple_intelligence.swift` (drop the `@Generable` macro that needs full Xcode) and adds a `LISPER_AI_STUB` build override. Workstream B adds an Ollama `PostProcessProvider`, settings, and a focused `ollama.rs` lifecycle module exposing Tauri commands + progress events. Workstream C adds an Ollama setup panel.

**Tech Stack:** Rust, Tauri 2, tauri-specta, reqwest, React 18, TypeScript, Tailwind, i18next, Bun. Ollama OpenAI-compatible API at `http://localhost:11434/v1`, native API at `/api/*`.

## Global Constraints

- **Build env (this machine):** `cmake` (4.x) installed; set `CMAKE_POLICY_VERSION_MINIMUM=3.5` for cargo. After Task 1, the default SDK works (NO `SDKROOT` override needed).
- **License:** keep `LICENSE` (MIT, © 2025 CJ Pais) + `NOTICE`.
- **App name:** `lisper` (lowercase) in user-facing strings; i18next for all new UI strings (eslint enforces — non-translatable literals via `{"..."}`).
- **Default Ollama model:** `llama3.2:3b`. **Default `num_ctx`:** `4096`. **Default post-process provider:** `ollama`.
- **Ollama provider:** `id="ollama"`, `label="Ollama (local)"`, `base_url="http://localhost:11434/v1"`, `allow_base_url_edit=true`, `models_endpoint=Some("/models")`, `supports_structured_output=false`.
- **Commit style:** conventional prefixes (`feat:`/`fix:`/`chore:`).
- **C ABI for Apple Intelligence is fixed** — only Swift internals change; do not touch `apple_intelligence.rs`/`actions.rs` Apple FFI calls.

---

## File Structure

**Created:**
- `src-tauri/src/ollama.rs` — Ollama lifecycle: detect/status/install/start/pull/ensure-ready + Tauri commands + progress events.
- `src/components/settings/PostProcessingSettingsApi/OllamaSetup.tsx` — setup panel (status chips, Set up button, model + context controls).

**Modified:**
- `src-tauri/swift/apple_intelligence.swift` — remove `@Generable`/`CleanedTranscript`, use plain `session.respond(to:)`.
- `src-tauri/build.rs` — honor `LISPER_AI_STUB=1` to force the stub.
- `src-tauri/src/settings.rs` — Ollama provider, default provider id, `ollama_model`/`ollama_num_ctx`/`ollama_auto_start` fields.
- `src-tauri/src/llm_client.rs` — optional `num_ctx` → `options.num_ctx` in the request for Ollama.
- `src-tauri/src/actions.rs` — pass `num_ctx` for the ollama provider into the chat call.
- `src-tauri/src/lib.rs` — `mod ollama;`, register ollama commands, auto-start hook in setup.
- `src/bindings.ts` — hand-add the new command stubs (specta will normalize on a real run).
- `src/components/settings/PostProcessingSettingsApi/` (provider settings) — render `OllamaSetup` when provider is `ollama`; hide the API-key field for Ollama.
- `src/i18n/locales/en/translation.json` — new keys.

---

# Workstream A — Apple Intelligence build fix

## Task 1: Fix the Swift + add stub override; build on default SDK

**Files:**
- Modify: `src-tauri/swift/apple_intelligence.swift`
- Modify: `src-tauri/build.rs`

**Interfaces:**
- Produces: a backend that compiles with the default macOS SDK under Command Line Tools (no `SDKROOT` hack); env `LISPER_AI_STUB=1` forces the stub.
- The C ABI is unchanged (`is_apple_intelligence_available`, `process_text_with_system_prompt_apple`, `AppleLLMResponse`, `free_apple_llm_response`).

- [ ] **Step 1: Remove the `@Generable` struct**

In `src-tauri/swift/apple_intelligence.swift`, delete the block:
```swift
@available(macOS 26.0, *)
@Generable
private struct CleanedTranscript: Sendable {
    let cleanedText: String
}
```

- [ ] **Step 2: Replace the structured-generation call with a plain text call**

In `processTextWithSystemPrompt`, replace this inner do/catch:
```swift
            do {
                let structured = try await session.respond(
                    to: swiftUserContent,
                    generating: CleanedTranscript.self
                )
                output = structured.content.cleanedText
            } catch {
                let fallbackGeneration = try await session.respond(to: swiftUserContent)
                output = fallbackGeneration.content
            }
```
with:
```swift
            let generation = try await session.respond(to: swiftUserContent)
            output = generation.content
```
(Leave everything else — the availability checks, semaphore, ResultBox, response marshaling — unchanged.)

- [ ] **Step 3: Verify the Swift compiles standalone (proves the fix)**

Run (default SDK, CLT):
```bash
cd src-tauri
SDK=$(xcrun --sdk macosx --show-sdk-path); SWIFTC=$(xcrun --find swiftc)
"$SWIFTC" -parse-as-library -target arm64-apple-macosx11.0 -sdk "$SDK" -O \
  -import-objc-header swift/apple_intelligence_bridge.h -c swift/apple_intelligence.swift -o /tmp/ai_check.o
echo "exit=$?"
```
Expected: `exit=0`, `/tmp/ai_check.o` created, no `@Generable`/`FoundationModelsMacros` errors.

- [ ] **Step 4: Add the `LISPER_AI_STUB` override in build.rs**

In `src-tauri/build.rs`, inside `build_apple_intelligence_bridge`, the source file is chosen by `has_foundation_models`. Add an explicit override just before that selection. Replace:
```rust
    let source_file = if has_foundation_models {
        println!("cargo:warning=Building with Apple Intelligence support.");
        REAL_SWIFT_FILE
    } else {
        println!("cargo:warning=Apple Intelligence SDK not found. Building with stubs.");
        STUB_SWIFT_FILE
    };
```
with:
```rust
    let force_stub = std::env::var("LISPER_AI_STUB").map(|v| v == "1").unwrap_or(false);
    println!("cargo:rerun-if-env-changed=LISPER_AI_STUB");
    let source_file = if force_stub {
        println!("cargo:warning=LISPER_AI_STUB=1 set. Building Apple Intelligence stub.");
        STUB_SWIFT_FILE
    } else if has_foundation_models {
        println!("cargo:warning=Building with Apple Intelligence support.");
        REAL_SWIFT_FILE
    } else {
        println!("cargo:warning=Apple Intelligence SDK not found. Building with stubs.");
        STUB_SWIFT_FILE
    };
```

- [ ] **Step 5: Verify the full backend builds on the DEFAULT SDK (no SDKROOT)**

Run:
```bash
cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo check 2>&1 | tail -8
```
Expected: `Finished` with no `swiftc failed`/`FoundationModels` errors. (The unrelated `block v0.1.6` future-incompat note is fine.) This is the headline: it must build WITHOUT `SDKROOT=...MacOSX15.4.sdk`.

- [ ] **Step 6: Verify the stub override also builds**

Run:
```bash
cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 LISPER_AI_STUB=1 cargo check 2>&1 | grep -E "stub|Finished" | head
```
Expected: prints the stub warning and `Finished`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/swift/apple_intelligence.swift src-tauri/build.rs
git commit -m "fix: compile Apple Intelligence without @Generable macro; add LISPER_AI_STUB override"
```

---

# Workstream B — Zero-touch Ollama (backend)

## Task 2: Ollama provider + settings fields

**Files:**
- Modify: `src-tauri/src/settings.rs` (provider list ~line 526, `default_post_process_provider_id` ~522, `AppSettings` struct ~392, defaults section)

**Interfaces:**
- Produces: settings `ollama_model: String` (default `"llama3.2:3b"`), `ollama_num_ctx: u32` (default `4096`), `ollama_auto_start: bool` (default `true`); default provider id `"ollama"`; an `ollama` entry in `default_post_process_providers()`.

- [ ] **Step 1: Add a unit test (TDD)**

Append to `src-tauri/src/settings.rs` test section (create `#[cfg(test)] mod ollama_settings_tests` if none):
```rust
#[cfg(test)]
mod ollama_settings_tests {
    use super::*;
    #[test]
    fn ollama_defaults_present() {
        assert_eq!(default_post_process_provider_id(), "ollama");
        assert_eq!(default_ollama_model(), "llama3.2:3b");
        assert_eq!(default_ollama_num_ctx(), 4096);
        assert!(default_ollama_auto_start());
        let providers = default_post_process_providers();
        let o = providers.iter().find(|p| p.id == "ollama").expect("ollama provider present");
        assert_eq!(o.base_url, "http://localhost:11434/v1");
        assert!(o.allow_base_url_edit);
        assert_eq!(o.models_endpoint.as_deref(), Some("/models"));
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test ollama_settings_tests 2>&1 | tail -6`
Expected: FAIL to compile — `default_ollama_model` etc. not found, provider missing.

- [ ] **Step 3: Add the Ollama provider and change the default**

In `default_post_process_providers()`, insert the Ollama provider as the FIRST element of the initial `vec![` (before OpenAI):
```rust
        PostProcessProvider {
            id: "ollama".to_string(),
            label: "Ollama (local)".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            allow_base_url_edit: true,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: false,
        },
```
Change `default_post_process_provider_id()` body to `"ollama".to_string()`.

- [ ] **Step 4: Add settings fields + default fns**

In the `AppSettings` struct (near the other `post_process_*` fields), add:
```rust
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_ollama_num_ctx")]
    pub ollama_num_ctx: u32,
    #[serde(default = "default_ollama_auto_start")]
    pub ollama_auto_start: bool,
```
Add the default fns near the other `default_*` fns:
```rust
fn default_ollama_model() -> String { "llama3.2:3b".to_string() }
fn default_ollama_num_ctx() -> u32 { 4096 }
fn default_ollama_auto_start() -> bool { true }
```
(If `AppSettings` derives `specta::Type`, these new fields will surface in bindings on a real run — fine.)

- [ ] **Step 5: Run the test to confirm it passes**

Run: `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test ollama_settings_tests 2>&1 | tail -6`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat: add Ollama provider as default + ollama_model/num_ctx/auto_start settings"
```

---

## Task 3: `ollama.rs` lifecycle module + commands

**Files:**
- Create: `src-tauri/src/ollama.rs`
- Modify: `src-tauri/src/lib.rs` (`mod ollama;` near other mods ~line 13; register commands in the `collect_commands![]` list ~line 334)

**Interfaces:**
- Consumes: `crate::settings::get_settings(&app)` for `ollama_model`.
- Produces Tauri commands (specta camelCases them): `ollama_status() -> OllamaStatus`, `ollama_start() -> Result<OllamaStatus,String>`, `ollama_install() -> Result<(),String>`, `ollama_pull(model: String) -> Result<(),String>`, `ollama_ensure_ready() -> Result<OllamaStatus,String>`. Emits events `ollama-log` (String) and `ollama-pull-progress` (`{model, status, percent}`).
- `OllamaStatus { installed: bool, running: bool, version: Option<String>, models: Vec<String>, has_model: bool }` (derives `serde::Serialize, specta::Type, Clone`).

- [ ] **Step 1: Create `src-tauri/src/ollama.rs`**

```rust
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
            if !p.is_empty() { return Some(p); }
        }
    }
    for c in ["/opt/homebrew/bin/ollama", "/usr/local/bin/ollama", "/usr/bin/ollama"] {
        if std::path::Path::new(c).exists() { return Some(c.to_string()); }
    }
    None
}

fn ollama_version(bin: &str) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// GET /api/tags — also serves as the running/health check.
fn fetch_models() -> Option<Vec<String>> {
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build().ok()?
        .get(format!("{}/api/tags", OLLAMA_HOST))
        .send().ok()?
        .text().ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let models = json.get("models")?.as_array()?
        .iter()
        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
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
    let has_model = models.iter().any(|m| m == &configured || m.starts_with(&format!("{}:", configured.split(':').next().unwrap_or(&configured))));
    OllamaStatus { installed, running, version, models, has_model }
}

#[tauri::command]
#[specta::specta]
pub fn ollama_status(app: AppHandle) -> OllamaStatus {
    status(&app)
}

#[tauri::command]
#[specta::specta]
pub fn ollama_install(app: AppHandle) -> Result<(), String> {
    if ollama_bin().is_some() { return Ok(()); }
    // macOS: prefer Homebrew if present. Otherwise instruct the UI.
    let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string());
    let brew = match brew {
        Some(b) => b,
        None => return Err("Homebrew not found. Please install Ollama from https://ollama.com/download".to_string()),
    };
    let _ = app.emit("ollama-log", "Installing Ollama via Homebrew…".to_string());
    let out = Command::new(brew).args(["install", "ollama"]).output()
        .map_err(|e| format!("brew failed to launch: {}", e))?;
    let _ = app.emit("ollama-log", String::from_utf8_lossy(&out.stdout).to_string());
    if !out.status.success() {
        return Err(format!("brew install ollama failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn ollama_start(app: AppHandle) -> Result<OllamaStatus, String> {
    if fetch_models().is_some() { return Ok(status(&app)); } // already running
    let bin = ollama_bin().ok_or("Ollama is not installed")?;
    // Spawn `ollama serve` detached; we don't hold the child (server is long-lived).
    Command::new(bin).arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn().map_err(|e| format!("Failed to start ollama serve: {}", e))?;
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
    let mut child = Command::new(bin).args(["pull", &model])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("Failed to start ollama pull: {}", e))?;
    // Ollama writes progress to stderr.
    if let Some(err) = child.stderr.take() {
        let reader = BufReader::new(err);
        for line in reader.lines().map_while(Result::ok) {
            // Lines look like "pulling manifest", "pulling abc... 45%". Extract a percent if present.
            let percent = line.split_whitespace()
                .find_map(|t| t.strip_suffix('%').and_then(|n| n.parse::<u32>().ok()));
            let _ = app.emit("ollama-pull-progress", serde_json::json!({
                "model": model, "status": line, "percent": percent
            }));
        }
    }
    let st = child.wait().map_err(|e| format!("pull wait failed: {}", e))?;
    if !st.success() { return Err(format!("ollama pull {} failed", model)); }
    let _ = app.emit("ollama-pull-progress", serde_json::json!({
        "model": model, "status": "complete", "percent": 100
    }));
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn ollama_ensure_ready(app: AppHandle) -> Result<OllamaStatus, String> {
    if ollama_bin().is_none() { ollama_install(app.clone())?; }
    if fetch_models().is_none() { ollama_start(app.clone())?; }
    let model = crate::settings::get_settings(&app).ollama_model;
    let s = status(&app);
    if !s.has_model { ollama_pull(app.clone(), model)?; }
    Ok(status(&app))
}

/// Best-effort background auto-start used at app launch.
pub fn auto_start_if_enabled(app: &AppHandle) {
    let settings = crate::settings::get_settings(app);
    if !settings.ollama_auto_start { return; }
    if ollama_bin().is_none() { return; }
    if fetch_models().is_some() { return; }
    let app2 = app.clone();
    std::thread::spawn(move || { let _ = ollama_start(app2); });
}
```

- [ ] **Step 2: Register the module + commands in lib.rs**

Add near the other `mod` lines: `mod ollama;`
In the `collect_commands![ ... ]` list (where `commands::overlay::dismiss_overlay_result` is), add:
```rust
            ollama::ollama_status,
            ollama::ollama_install,
            ollama::ollama_start,
            ollama::ollama_pull,
            ollama::ollama_ensure_ready,
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo check 2>&1 | tail -8`
Expected: `Finished`, no errors. (`reqwest` `blocking` feature: if a compile error says the blocking client/feature is missing, enable it — check `Cargo.toml` for `reqwest` and add `"blocking"` to its `features`, then note this in the report.)

- [ ] **Step 4: Smoke-test the lifecycle on this machine (ollama is installed)**

```bash
cd src-tauri
# Confirm detection + start works via a tiny throwaway check using the same logic:
which ollama && (curl -s -m 1 http://localhost:11434/api/tags >/dev/null && echo "already running" || (ollama serve >/dev/null 2>&1 & sleep 2; curl -s -m 2 http://localhost:11434/api/tags | head -c 80; echo))
```
Expected: server reachable (JSON with `models`). Report the output. (This validates the detection/health approach the Rust uses.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ollama.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add Ollama lifecycle module (status/install/start/pull/ensure-ready) + commands"
```

---

## Task 4: Context-length wiring (num_ctx) for Ollama

**Files:**
- Modify: `src-tauri/src/llm_client.rs` (`ChatCompletionRequest` ~line 36; both `send_chat_completion` and `send_chat_completion_with_schema`)
- Modify: `src-tauri/src/actions.rs` (the two `send_chat_completion*` call sites ~lines 221, 279)

**Interfaces:**
- Consumes: `provider.id`, `settings.ollama_num_ctx`.
- Produces: requests to the Ollama provider include `"options": {"num_ctx": N}`; other providers unchanged. New trailing param `num_ctx: Option<u32>` on both client fns.

- [ ] **Step 1: Add the options struct + request field**

In `llm_client.rs`, add near `ReasoningConfig`:
```rust
#[derive(Debug, Serialize)]
struct OllamaOptions {
    num_ctx: u32,
}
```
Add to `ChatCompletionRequest`:
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
```

- [ ] **Step 2: Thread `num_ctx` through both client fns**

Add a trailing `num_ctx: Option<u32>` parameter to `send_chat_completion_with_schema` and to `send_chat_completion` (which forwards it). In `send_chat_completion`'s call to `_with_schema`, pass the new `num_ctx` through. In `_with_schema`, build the field:
```rust
        options: num_ctx.map(|n| OllamaOptions { num_ctx: n }),
```
and add it to the `ChatCompletionRequest { ... }` literal.

In `send_chat_completion` (the thin wrapper), update its signature to accept `num_ctx: Option<u32>` and forward it as the final arg of `_with_schema`.

- [ ] **Step 3: Pass num_ctx from the call sites**

In `actions.rs`, before the calls, compute:
```rust
        let ollama_num_ctx = if provider.id == "ollama" {
            Some(crate::settings::get_settings(app).ollama_num_ctx)
        } else { None };
```
(Use the `app: &AppHandle` already in scope in this function — confirm the variable name; the function building the request has access to settings already via the provider lookup.) Add `ollama_num_ctx` (or `.clone()` if needed) as the new trailing argument to BOTH `send_chat_completion_with_schema(...)` and `send_chat_completion(...)` calls.

- [ ] **Step 4: Verify compile**

Run: `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo check 2>&1 | tail -6`
Expected: `Finished`, no errors (all call sites updated).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/llm_client.rs src-tauri/src/actions.rs
git commit -m "feat: send num_ctx option to Ollama for configurable context length"
```

---

## Task 5: Auto-start Ollama on app launch

**Files:**
- Modify: `src-tauri/src/lib.rs` (the `.setup(|app| { ... })` closure)

**Interfaces:**
- Consumes: `crate::ollama::auto_start_if_enabled(&AppHandle)`.

- [ ] **Step 1: Call auto-start in setup**

In the Tauri `.setup(...)` closure in `lib.rs`, after managers/settings are initialized, add:
```rust
            crate::ollama::auto_start_if_enabled(&app.handle());
```
(Place it near the end of setup so settings are loaded. `app.handle()` yields the `AppHandle`.)

- [ ] **Step 2: Verify compile**

Run: `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo check 2>&1 | tail -6`
Expected: `Finished`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: auto-start managed Ollama server on launch when enabled"
```

---

# Workstream C — Ollama setup UI + polish

## Task 6: Ollama setup panel

**Files:**
- Create: `src/components/settings/PostProcessingSettingsApi/OllamaSetup.tsx`
- Modify: provider settings render (the component that shows per-provider config — `src/components/settings/PostProcessingSettingsApi/` index/ProviderSelect or the parent that renders provider details) to show `OllamaSetup` when the selected provider id is `ollama`, and hide the API-key input for `ollama`.
- Modify: `src/bindings.ts` (hand-add the 5 ollama command stubs)
- Modify: `src/i18n/locales/en/translation.json`

**Interfaces:**
- Consumes: `commands.ollamaStatus()`, `ollamaEnsureReady()`, `ollamaPull(model)`, `ollamaStart()`, `ollamaInstall()`; events `ollama-pull-progress`, `ollama-log`, `ollama-status`; settings `ollama_model`, `ollama_num_ctx` via the existing settings hook.

- [ ] **Step 1: Hand-add command stubs to bindings.ts**

Find the `commands` object in `src/bindings.ts` and add (matching the existing no-arg / arg patterns, e.g. how `playTestSound` / `dismissOverlayResult` are written):
```ts
async ollamaStatus() : Promise<OllamaStatus> {
    return await TAURI_INVOKE("ollama_status");
},
async ollamaInstall() : Promise<Result<null, string>> {
    try { return { status: "ok", data: await TAURI_INVOKE("ollama_install") }; }
    catch (e) { if(e instanceof Error) throw e; else return { status: "error", error: e as any }; }
},
async ollamaStart() : Promise<Result<OllamaStatus, string>> {
    try { return { status: "ok", data: await TAURI_INVOKE("ollama_start") }; }
    catch (e) { if(e instanceof Error) throw e; else return { status: "error", error: e as any }; }
},
async ollamaPull(model: string) : Promise<Result<null, string>> {
    try { return { status: "ok", data: await TAURI_INVOKE("ollama_pull", { model }) }; }
    catch (e) { if(e instanceof Error) throw e; else return { status: "error", error: e as any }; }
},
async ollamaEnsureReady() : Promise<Result<OllamaStatus, string>> {
    try { return { status: "ok", data: await TAURI_INVOKE("ollama_ensure_ready") }; }
    catch (e) { if(e instanceof Error) throw e; else return { status: "error", error: e as any }; }
},
```
Add a matching type:
```ts
export type OllamaStatus = { installed: boolean; running: boolean; version: string | null; models: string[]; has_model: boolean };
```
(Match the `Result<...>` helper shape already used in this file — copy an existing fallible command's exact wrapper if it differs. A real `tauri dev` run regenerates this file canonically.)

- [ ] **Step 2: Add i18n keys**

In `en/translation.json`, add an `ollama` object under the post-processing area:
```json
"ollama": {
  "title": "Ollama (local)",
  "setup": "Set up Ollama",
  "installed": "Installed",
  "running": "Server running",
  "modelReady": "Model ready",
  "model": "Model",
  "contextLength": "Context length",
  "pull": "Download model",
  "start": "Start server",
  "working": "Working…"
}
```

- [ ] **Step 3: Create `OllamaSetup.tsx`**

```tsx
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, OllamaStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";

const CTX_OPTIONS = [2048, 4096, 8192, 16384];

const Chip: React.FC<{ ok: boolean; label: string }> = ({ ok, label }) => (
  <span
    className={`rounded-full px-2 py-0.5 text-xs ${ok ? "bg-logo-primary text-white" : "bg-mid-gray/30 text-text"}`}
  >
    {label}
  </span>
);

export const OllamaSetup: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const [status, setStatus] = useState<OllamaStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<string>("");

  const refresh = async () => setStatus(await commands.ollamaStatus());

  useEffect(() => {
    refresh();
    const unP = listen<{ status: string; percent: number | null }>(
      "ollama-pull-progress",
      (e) => setProgress(`${e.payload.status}${e.payload.percent != null ? ` ${e.payload.percent}%` : ""}`),
    );
    const unL = listen<string>("ollama-log", (e) => setProgress(e.payload));
    return () => { unP.then((f) => f()); unL.then((f) => f()); };
  }, []);

  const setup = async () => {
    setBusy(true);
    try { await commands.ollamaEnsureReady(); await refresh(); }
    finally { setBusy(false); setProgress(""); }
  };

  const model = (getSetting("ollama_model") as string) ?? "llama3.2:3b";
  const numCtx = (getSetting("ollama_num_ctx") as number) ?? 4096;

  return (
    <SettingContainer title={t("ollama.title")} description="">
      <div className="flex flex-col gap-3">
        <div className="flex gap-2">
          <Chip ok={!!status?.installed} label={t("ollama.installed")} />
          <Chip ok={!!status?.running} label={t("ollama.running")} />
          <Chip ok={!!status?.has_model} label={t("ollama.modelReady")} />
        </div>

        <div className="flex items-center gap-2">
          <Button onClick={setup} disabled={busy}>
            {busy ? t("ollama.working") : t("ollama.setup")}
          </Button>
          {progress && <span className="text-xs text-mid-gray">{progress}</span>}
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm">{t("ollama.model")}</label>
          <Dropdown
            selectedValue={model}
            onSelect={(v) => updateSetting("ollama_model", v)}
            options={(status?.models?.length ? status.models : [model]).map((m) => ({ value: m, label: m }))}
          />
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm">{t("ollama.contextLength")}</label>
          <Dropdown
            selectedValue={String(numCtx)}
            onSelect={(v) => updateSetting("ollama_num_ctx", Number(v))}
            options={CTX_OPTIONS.map((n) => ({ value: String(n), label: String(n) }))}
          />
        </div>
      </div>
    </SettingContainer>
  );
};
```
(If `Dropdown`/`Button`/`SettingContainer` props differ from this usage, adapt to their real signatures — check `src/components/ui/`. `getSetting`/`updateSetting` come from `useSettings`; if the settings keys aren't yet in the generated `AppSettings` TS type, a real run regenerates them — hand-widen the type if the build complains.)

- [ ] **Step 4: Render it for the Ollama provider + hide the key field**

In the provider-details render (find it: `grep -rn "allow_base_url_edit\|api_key\|apiKey\|provider.id\|providerId" src/components/settings/PostProcessingSettingsApi`), when the selected provider id is `"ollama"`: render `<OllamaSetup />` and do NOT render the API-key text input (Ollama needs no key). Keep base-url editing available (allow_base_url_edit is true).

- [ ] **Step 5: Verify build + lint**

Run: `bun run build && bun run lint` (or `./node_modules/.bin/tsc --noEmit && ./node_modules/.bin/eslint src`)
Expected: type-check + lint clean. Fix any prop mismatches against the real UI components.

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/PostProcessingSettingsApi src/bindings.ts src/i18n/locales/en/translation.json
git commit -m "feat: Ollama setup panel (status chips, one-click setup, model + context controls)"
```

---

## Task 7: Polish pass

**Files:**
- Modify: provider settings (ensure Ollama preselected reads well; provider label shows "Ollama (local)")
- Modify: `src/overlay/RecordingOverlay.css` (minor result-panel spacing/contrast)

**Interfaces:** none new.

- [ ] **Step 1: Verify provider default + label**

Run the app's provider selector logic: confirm `ollama` is the default selected provider and its label "Ollama (local)" displays. If the selector reads a hardcoded default elsewhere in the frontend, align it to read from settings.

- [ ] **Step 2: Minor overlay result-panel polish**

In `RecordingOverlay.css`, tighten the `.result-panel`/`.result-text`/`.result-copy` rules for readability on the violet theme (e.g. ensure adequate contrast and 8px padding; round the copy button; cap text width). Keep changes small and cosmetic.

- [ ] **Step 3: Verify build + lint**

Run: `./node_modules/.bin/tsc --noEmit && ./node_modules/.bin/eslint src`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/components src/overlay
git commit -m "polish: Ollama provider default display and overlay result-panel styling"
```

---

# Final verification
- [ ] `cd src-tauri && CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build 2>&1 | tail -5` → builds on DEFAULT SDK (no SDKROOT), Apple Intelligence real path compiled.
- [ ] `cargo test 2>&1 | tail -5` → ollama_settings_tests + sound_theme_tests pass.
- [ ] `./node_modules/.bin/tsc --noEmit && ./node_modules/.bin/eslint src` → clean.
- [ ] On the dev machine: `commands.ollamaEnsureReady()` path validated (server starts, model pulls) — or the Step-4 smoke test in Task 3 confirms detection/health.
- [ ] LICENSE + NOTICE intact.

## Self-review notes (author)
- Spec §1 (A1 Swift fix, A2 stub flag, A3 builds on default SDK) → Task 1. ✓ (empirically validated the rewrite compiles)
- Spec §2 B1 provider/default → Task 2; B2 settings → Task 2; B3 lifecycle+commands → Task 3; B4 num_ctx → Task 4; B3 auto-start → Task 5. ✓
- Spec §3 C1 panel → Task 6; C2 polish → Task 7; C3 i18n → Task 6. ✓
- Spec §5 out-of-scope respected (no binary bundling, no pull-cancel, macOS/brew-only auto-install). ✓
- Naming consistency: `ollama_status/ollama_start/ollama_install/ollama_pull/ollama_ensure_ready` (Rust) ↔ `ollamaStatus/...` (TS); `OllamaStatus` fields `installed/running/version/models/has_model`; events `ollama-pull-progress`/`ollama-log`/`ollama-status` identical in Rust emit and TS listen. ✓
- Known risk: `reqwest` blocking feature may need enabling (Task 3 Step 3 handles); bindings.ts hand-edits normalized by a real run.
