# lisper — Design Spec

**Date:** 2026-06-25
**Status:** Approved (pending final spec review)
**Summary:** Fork and fully rebrand the MIT-licensed Handy speech-to-text app into **lisper** (robot logo, violet color scheme, own bundle identity), and add two Wispr-Flow-style features: a deep "click" sound on mic activation and an expandable copyable result panel on the recording overlay.

---

## 1. Context

Handy is a Tauri 2.x desktop speech-to-text app: Rust backend (`src-tauri/`) + React/TypeScript frontend (`src/`), Tailwind v4 styling driven by CSS variables, i18next for all user-facing strings. It is MIT-licensed (`Copyright (c) 2025 CJ Pais`) and explicitly designed to be forked.

This project is a **full-ownership fork**: rebrand identity, change bundle identifier and product names, and add new functionality. Visual polish is the top priority — the rebrand should look finished — and all three workstreams ship together in one implementation plan.

### License compliance (non-negotiable)
- Keep the MIT `LICENSE` file and the `Copyright (c) 2025 CJ Pais` notice intact.
- Add a short `NOTICE` file stating lisper is a fork of Handy and retaining the upstream attribution.
- Renaming the product, identifiers, and assets is permitted under MIT.

---

## 2. Workstream A — Rebrand (Handy → lisper)

### A1. Identity & ownership
- `src-tauri/tauri.conf.json`:
  - `productName`: `Handy` → `lisper`
  - `identifier`: `com.pais.handy` → `com.lisper.app` (a bundle ID is a unique reverse-DNS string; it does not require owning a domain)
  - `bundle.windows.signCommand`: remove the upstream Azure signing command (no signing infra in scope)
  - `plugins.updater.endpoints`: point away from `cjpais/Handy`. For now, **disable the updater** (or set a placeholder endpoint) since there is no release host yet. Keep `pubkey` only if updater stays enabled — otherwise remove.
- `package.json`: `name` `handy-app` → `lisper-app`.
- `src-tauri/Cargo.toml`: bin `handy` → `lisper`, lib `handy_app_lib` → `lisper_app_lib`. Update references in `main.rs`/`lib.rs` and anywhere the lib name is used.

### A2. User-facing strings
There are ~399 case-insensitive "handy" matches, but most are the English adjective, code identifiers, or upstream URLs. Scope of change:
- **Brand-name strings only** in `src/i18n/locales/en/translation.json` (source of truth) and the brand-name occurrences across the other locale files (~30 languages) — onboarding, About screen, window/tray titles, tooltips.
- Window title / tray tooltip set in Rust (`overlay.rs`, `tray.rs`, `lib.rs`).
- Do **not** mass-replace the adjective "handy" or upstream documentation/URLs that are factual references.

### A3. Code identifiers (cleanliness)
- `src/components/icons/HandyTextLogo.tsx` → `LisperTextLogo.tsx`
- `src/components/icons/HandyHand.tsx` → `LisperBot.tsx` (new robot mark)
- Update `src/components/icons/index.ts` exports and all import sites.
- `HandyKeysShortcutInput.tsx` keeps its name (it refers to a key-handling implementation, not the brand) — confirm during implementation, rename only if it is brand-derived.

### A4. Robot logo (generated SVG)
- Design an original robot SVG (head/antenna/eyes), violet-themed, simple enough to read at 16px tray size.
- New `LisperBot` React component (replaces `HandyHand` usage in sidebar, onboarding, About).
- New `LisperTextLogo` wordmark ("lisper").
- Render the SVG to every icon slot:
  - `src-tauri/icons/`: `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.icns`, `icon.ico`, `logo.png`, all `Square*Logo.png`, `StoreLogo.png`, and the `ios/` set.
  - Tray PNGs in `src-tauri/resources/`: `tray_idle.png`, `tray_idle_dark.png`, `tray_recording.png`, `tray_recording_dark.png`, `tray_transcribing.png`, `tray_transcribing_dark.png`, plus `handy.png`/`recording.png`/`transcribing.png` (rename/replace).
- Use a generation tool (e.g. an SVG→PNG rasterizer + `iconutil`/`png2icns` for `.icns`, and an `.ico` packer) to produce all sizes from one source SVG.

### A5. Color scheme → Violet `#7C3AED`
- Current palette in `src/App.css` is pink (`--color-background-ui: #da5893`, `--color-logo-primary: #faa2ca`, etc.) with a dark-mode block.
- Replace with a coordinated violet palette anchored on `#7C3AED`:
  - Light mode: `--color-logo-primary` ≈ `#a78bfa`, `--color-background-ui` ≈ `#7C3AED`, strokes tuned for contrast.
  - Dark mode: lighter violet logo primary, dark violet-tinted background.
- Tailwind (`tailwind.config.js`) already maps these CSS vars to utility classes, so most components recolor automatically. Audit `RecordingOverlay.css` and any hardcoded hex values for stragglers.

---

## 3. Workstream B — Deep "click" sound on mic activation

Handy already has a sound-theme system: `SoundTheme { Marimba, Pop, Custom }` in `settings.rs`, each resolving to `resources/<theme>_start.wav` / `_stop.wav`, played via `audio_feedback.rs` (rodio) with volume control, output-device selection, a `play_test_sound` command, and a frontend theme selector.

### Changes
- `settings.rs`: add `SoundTheme::Lisper` variant; map `as_str()` → `"lisper"`; set `default_sound_theme()` → `SoundTheme::Lisper`.
- Generate placeholder WAVs now: `resources/lisper_start.wav` (deep, satisfying click) and `resources/lisper_stop.wav` (softer release tick), synthesized programmatically. These are replaceable later; the `Custom` theme already supports user-supplied sounds.
- Frontend: add the `Lisper` option to the sound-theme selector component + i18n label.
- No changes to playback plumbing — reuse `play_feedback_sound` / volume / device logic.

The activation "click" is the existing **start** feedback sound; we are supplying a deep-click WAV for it as the new default theme. No separate UI-sound subsystem is introduced (keeps scope tight).

---

## 4. Workstream C — Expandable copyable result panel

Currently the overlay is a fixed **172×36** window (`overlay.rs`) that renders an icon + mic-level bars + cancel button (`RecordingOverlay.tsx`) and simply hides when done. Final transcription text is produced by `transcribe()` and pasted into the active app; it is **not** currently sent to the overlay.

### Behavior
After transcription completes, the overlay bar **expands** into a panel showing the transcribed text with a **Copy** button. The panel **auto-hides after ~5 seconds, but the timer pauses while the pointer is hovering/interacting** with it. Paste-into-active-app behavior is unchanged; the panel is additive.

### Backend (`src-tauri/src`)
- In the post-transcription path (`actions.rs` / `managers/transcription.rs`), emit a new event `transcription-result` (payload: final text) to the `recording_overlay` window via `emit_to`.
- Add `resize_overlay(app, width, height)` in `overlay.rs` to grow the window to a result size (e.g. ~360×120, clamped to monitor) and restore to `OVERLAY_WIDTH×OVERLAY_HEIGHT` on dismiss. Reposition so it stays anchored per `OverlayPosition`.
- Add a `hide`/`reset-overlay` trigger the frontend can call when the auto-hide timer fires or the user dismisses.

### Frontend (`RecordingOverlay.tsx`)
- Add overlay state `"result"`. On `transcription-result`: store text, switch to `"result"`, request resize.
- Render the text (scrollable if long) + a Copy button using the existing clipboard plugin (`@tauri-apps/plugin-clipboard-manager`).
- Auto-hide: ~5s timer; `onMouseEnter` clears it, `onMouseLeave` restarts it. Esc / click-away also dismisses. On dismiss → request reset to bar size + hide.
- i18n keys for "Copy" / "Copied".

### Edge cases
- Empty transcription: skip the result panel (upstream already skips post-processing on empty).
- Very long text: cap panel height, make body scrollable.
- Overlay disabled in settings: keep current behavior, no result panel.

---

## 5. Ordering & isolation

All three ship in one plan, but sequence for a good-looking, independently-usable result:
1. **Workstream A (rebrand)** first — identity, logo, colors. Produces a visibly finished "lisper".
2. **Workstream B (sound)** — small, self-contained.
3. **Workstream C (overlay panel)** — the only genuinely new subsystem (window resize + new state + event), built last.

Each workstream is independently testable. A and B touch config/assets/existing systems (low risk); C adds new event wiring and window-resize logic (higher risk, isolated to overlay).

---

## 6. Out of scope
- Code-signing / notarization infrastructure and certificates.
- Hosting an actual update server / publishing releases (config is pointed away from upstream; no new host stood up).
- Translating new strings into all locales beyond English (English source + existing-key reuse; other locales fall back to English for new keys).
- Any change to the transcription pipeline, models, or paste behavior.

## 7. Success criteria
- App builds and runs as "lisper" with violet UI and robot icons in app window, dock/taskbar, and tray.
- No remaining brand references to "Handy" in user-facing UI (adjective/doc/URL references excepted).
- Default sound theme is the deep-click "lisper" theme; selectable and test-playable in settings.
- After dictation, the overlay expands to show copyable text, Copy works, and it auto-hides after ~5s unless hovered.
- MIT license + upstream attribution retained; `NOTICE` added.
