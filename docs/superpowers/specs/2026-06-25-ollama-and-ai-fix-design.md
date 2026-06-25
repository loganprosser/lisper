# lisper — Apple Intelligence build fix + zero-touch Ollama — Design Spec

**Date:** 2026-06-25
**Status:** Auto-approved by user ("just implement this and continue") — proceeding autonomously; documented for later review.
**Summary:** Three workstreams on the `lisper` fork: (1) fix the Apple Intelligence Swift so the backend compiles on Command Line Tools (no full Xcode / no SDKROOT hack) plus an explicit stub-fallback flag; (2) make Ollama a zero-touch local LLM — the app installs (if missing), launches, pulls a model, sets context, and uses it by default so the user never touches Ollama; (3) refine the UI/UX, focused on the Ollama setup flow and rebrand polish.

---

## 0. Context & decisions

The post-processing LLM system uses `PostProcessProvider { id, label, base_url, allow_base_url_edit, models_endpoint, supports_structured_output }` (settings.rs) and an OpenAI-compatible client (llm_client.rs). Apple Intelligence is a special provider handled via Swift FFI (apple_intelligence.rs ↔ swift/apple_intelligence.swift). Ollama serves an OpenAI-compatible API at `http://localhost:11434/v1` and a native API at `/api/*`.

Empirically validated before writing this spec:
- Real cause of the AI build failure: `@Generable` needs the `FoundationModelsMacros` plugin, which ships only with full Xcode; under Command Line Tools the macro can't expand → conformance error.
- A rewrite dropping `@Generable`/`CleanedTranscript` and using plain `session.respond(to:)` **compiles cleanly** against the default SDK with CLT (tested).
- `ollama` is already installed on the dev machine (`/opt/homebrew/bin/ollama`), server not running.

Locked decisions (made autonomously; revisit if undesired):
- Default Ollama model: **`llama3.2:3b`** (small, fast, good for transcript cleanup; ~2GB).
- Default `num_ctx`: **4096**.
- Default post-process provider → **`ollama`** (but post-processing only runs if the user enables it; we do not silently force LLM calls).
- Ollama provider `supports_structured_output: false` (safe; cleanup is prompt-driven).
- Auto-install strategy on macOS: prefer `brew install ollama` if Homebrew is present; otherwise surface a one-click link / instructions (no silent curl|sh). The managed lifecycle (start/pull/use) works regardless.

---

## 1. Workstream A — Apple Intelligence build fix

### A1. Swift rewrite (the actual fix)
`swift/apple_intelligence.swift`: remove the `@available(macOS 26.0,*) @Generable private struct CleanedTranscript` and replace the structured `session.respond(to:generating:CleanedTranscript.self)` do/catch with a single plain call:
```swift
let generation = try await session.respond(to: swiftUserContent)
output = generation.content
```
The C ABI (`is_apple_intelligence_available`, `process_text_with_system_prompt_apple`, `AppleLLMResponse`, `free_apple_llm_response`) is unchanged, so `apple_intelligence.rs` and `actions.rs` are untouched. Cleanup quality now comes entirely from the system prompt (`build_system_prompt`), which is already passed as `instructions`.

Update `swift/apple_intelligence_stub.swift` only if it referenced the removed symbols (it should already be a pure stub — verify).

### A2. Explicit stub-fallback flag (build.rs)
`build.rs` already auto-selects the stub when `FoundationModels.framework` is absent. Add an explicit override: if env `LISPER_AI_STUB=1` is set, force the stub regardless of SDK. This guarantees a buildable backend on any machine/CI without depending on SDK contents, and documents the escape hatch. Keep the existing panic-on-real-compile-failure (we do not want to silently ship a no-op main), now unreachable on CLT because A1 compiles.

### A3. Success criteria
`cd src-tauri && cargo check` and `cargo build` succeed with the **default** SDK (no `SDKROOT` override, no cmake-policy issue aside from the unrelated `CMAKE_POLICY_VERSION_MINIMUM=3.5` whisper requirement). Apple Intelligence remains available at runtime on capable devices.

---

## 2. Workstream B — Zero-touch Ollama (backend)

### B1. Provider registration
In `default_post_process_providers()` add:
```
PostProcessProvider { id: "ollama", label: "Ollama (local)",
  base_url: "http://localhost:11434/v1", allow_base_url_edit: true,
  models_endpoint: Some("/models"), supports_structured_output: false }
```
Insert near the top so it reads as the primary local option. Change `default_post_process_provider_id()` → `"ollama"`. No API key required (existing `build_headers` skips auth when key is empty).

### B2. Settings additions
- `ollama_model: String` (default `"llama3.2:3b"`) — the model lisper manages/uses.
- `ollama_num_ctx: u32` (default `4096`).
- `ollama_auto_start: bool` (default `true`) — start the server on app launch if installed.

### B3. `src-tauri/src/ollama.rs` lifecycle manager + Tauri commands
A focused module that owns Ollama process/state. Uses `std::process::Command`/Tauri shell and `reqwest` for health.
- `detect()` → resolves the `ollama` binary path (PATH + `/opt/homebrew/bin`, `/usr/local/bin`, `/Applications/Ollama.app/...`), returns `installed: bool` + version.
- `is_running()` → GET `http://localhost:11434/api/tags` (short timeout) → bool.
- `list_models()` → GET `/api/tags` → `Vec<String>` of installed model tags.
- Commands (registered in lib.rs command list, specta-exposed → camelCase bindings):
  - `ollama_status()` → `{ installed, running, version: Option<String>, models: Vec<String>, hasModel: bool }` (hasModel = configured `ollama_model` present).
  - `ollama_install()` → if not installed: run `brew install ollama` when `brew` exists (stream stdout/stderr as `ollama-log` events); otherwise return an error payload telling the UI to show the download link. Idempotent.
  - `ollama_start()` → if not running: spawn `ollama serve` detached/managed; poll health up to ~10s; emit `ollama-status` when up. Idempotent (no-op if already running).
  - `ollama_pull(model)` → spawn `ollama pull <model>`; parse progress lines; emit `ollama-pull-progress` events `{ model, status, percent? }`; resolve on completion.
  - `ollama_ensure_ready()` → orchestrates detect → install (if brew) → start → pull(configured model if missing) → returns final `ollama_status`. This is the single zero-touch entry point the UI's primary button calls.
- Auto-start hook: in lib.rs setup, if `ollama_auto_start` and installed and not running, fire `ollama_start()` in the background (non-blocking, best-effort).

### B4. Context length wiring (llm_client.rs)
When `provider.id == "ollama"`, include an `options` object with `num_ctx` in the chat-completion request body (extra field; Ollama's server honors `options.num_ctx`). Thread the value from settings to the call site in actions.rs. If a given Ollama version ignores it, behavior degrades gracefully to the model default — documented as a soft feature.

### B5. Edge cases
- Ollama not installed and no brew → `ollama_status.installed=false`; UI shows install guidance; nothing crashes.
- Server start failure / port busy → surfaced as an error event; provider still selectable (user may run their own).
- Pull of a large model → progress streamed; cancellable is out of scope (YAGNI) but the UI must show it's working.
- Managed `ollama serve` child should be terminated on app exit if we started it (track ownership; don't kill a user-started server).

---

## 3. Workstream C — UI/UX

### C1. Ollama setup panel (new, in post-process settings)
A component shown when the Ollama provider is selected (and surfaced in onboarding if post-processing is enabled):
- Status row: three chips — **Installed**, **Server running**, **Model ready** — each green/amber with live state from `ollama_status` (refreshed on mount + after actions + via `ollama-status` events).
- Primary button **"Set up Ollama"** → calls `ollama_ensure_ready()`; shows inline progress (install → starting → pulling `llama3.2:3b` NN%) driven by `ollama-log` / `ollama-pull-progress` events. Disabled→spinner while running.
- **Model** dropdown: lists local models (`ollama_status.models`); selecting updates `ollama_model`. A "Pull another model…" affordance accepts a tag and calls `ollama_pull`.
- **Context length** control (e.g. select: 2048 / 4096 / 8192 / 16384) bound to `ollama_num_ctx`.
- **Server** toggle: Start/Stop the managed server (Stop only if we started it).
- No API-key field for Ollama (hide it; the provider needs none).

### C2. Rebrand/polish pass
- Ensure the violet theme reads well on the new panel; use existing UI primitives (`SettingContainer`, `Dropdown`, `Button`, status chips) for consistency.
- Verify the Ollama provider appears first and is preselected; the post-process provider selector shows "Ollama (local)" cleanly.
- Light polish on the overlay result panel (spacing/contrast) and onboarding copy where it references the LLM step — keep scope tight, no redesign.

### C3. i18n
All new user-facing strings use i18next keys in `en/translation.json` (eslint enforces). New keys only added to English; other locales fall back.

---

## 4. Ordering, isolation, verification
1. **Workstream A first** — unblocks normal builds + lets us actually run the app to verify B/C. Verify: `cargo build` on default SDK; re-test the swiftc compile is wired through build.rs.
2. **Workstream B** — backend provider + lifecycle + settings + context. Verify: `cargo check` clean; on the dev machine actually exercise `ollama_start`/`ollama_pull`/`list_models` (ollama is installed).
3. **Workstream C** — UI. Verify: `bun run build` + `eslint` clean; ideally a real `tauri dev` run to see the panel + theme.

Each workstream is independently shippable. A is pure build/Swift; B is backend Rust + settings; C is frontend. Files are focused: new `ollama.rs` owns lifecycle; the setup panel is its own component.

## 5. Out of scope
- Bundling the Ollama binary inside the app installer (we manage an installed/brew Ollama, not redistribute it).
- Pull cancellation/pause; multi-model management UI beyond pick + pull.
- Windows/Linux Ollama auto-install (detect + start + pull still work cross-platform where `ollama` is on PATH; auto-install path is macOS/brew-only this round).
- Changing the transcription engine or non-Ollama providers' behavior.

## 6. Success criteria
- App builds and runs on this Mac with the default SDK (no SDKROOT hack); Apple Intelligence still available at runtime.
- With Ollama installed, a single "Set up Ollama" click starts the server, pulls `llama3.2:3b`, and post-processing (when enabled) routes through local Ollama by default.
- Status chips reflect reality; pulling shows progress; selecting a model and context length persists.
- Frontend builds, lints clean; backend compiles clean; MIT/NOTICE intact.
