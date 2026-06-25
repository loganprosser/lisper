# lisper Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fork the MIT-licensed Handy app into a fully rebranded "lisper" (robot logo, violet UI, own identity) and add a deep mic-activation click sound plus an expandable copyable result panel on the recording overlay.

**Architecture:** Tauri 2.x app — Rust backend (`src-tauri/`) + React/TS frontend (`src/`). Rebrand touches config, assets, CSS variables (Tailwind reads them), and i18n strings. The sound feature extends the existing `SoundTheme` enum + asset resolution. The overlay panel adds one new Rust event + window-resize fn and a new frontend overlay state.

**Tech Stack:** Rust, Tauri 2, React 18, TypeScript, Tailwind v4, i18next, rodio (audio), Bun, Playwright. Icon generation via `rsvg-convert`/`sips`/`iconutil` (macOS) or `magick`.

## Global Constraints

- **License:** Keep `LICENSE` (MIT) and `Copyright (c) 2025 CJ Pais` intact. Add a `NOTICE` file crediting Handy as the upstream fork source. (Spec §1)
- **App name:** `lisper` (lowercase) everywhere user-facing. (Spec §2)
- **Bundle ID:** `com.lisper.app`. (Spec §2.A1)
- **Anchor color:** Violet `#7C3AED`. (Spec §2.A5)
- **i18n rule:** No hardcoded user-facing strings in JSX — ESLint enforces. New text → add key to `src/i18n/locales/en/translation.json`, use via `t('key')`. (AGENTS.md)
- **Default sound theme:** `Lisper` (deep click). (Spec §3)
- **Result panel dismiss:** auto-hide ~5s, timer pauses while hovered/interacting. (Spec §4)
- **Out of scope:** code signing, hosting an update server, translating new keys beyond English. (Spec §6)
- **Commit style:** conventional prefixes (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`). (AGENTS.md)
- **Verify formatting before commit:** `bun run format` (Prettier + cargo fmt).

---

## File Structure

**Created:**
- `NOTICE` — upstream attribution.
- `src/components/icons/LisperBot.tsx` — robot logo React component (replaces `HandyHand`).
- `src/components/icons/LisperTextLogo.tsx` — "lisper" wordmark (replaces `HandyTextLogo`).
- `assets/branding/lisper-bot.svg` — source SVG for the robot, used to generate all raster icons.
- `src-tauri/resources/lisper_start.wav`, `src-tauri/resources/lisper_stop.wav` — generated placeholder sounds.
- `scripts/gen-icons.sh` — regenerates all icon rasters from the source SVG.
- `scripts/gen-sounds.ts` — generates placeholder WAVs.

**Modified:**
- `src-tauri/tauri.conf.json` — productName, identifier, updater, signing.
- `package.json` — name.
- `src-tauri/Cargo.toml` — bin/lib names.
- `src-tauri/src/settings.rs` — `SoundTheme::Lisper` + default.
- `src-tauri/src/audio_feedback.rs` — custom-theme match arm (no logic change, verify).
- `src-tauri/src/overlay.rs` — `resize_overlay` + reset-to-bar size; default-size constants reused.
- `src-tauri/src/actions.rs` — emit `transcription-result`, defer hide for result panel.
- `src-tauri/src/commands/overlay.rs` (or nearest existing overlay command module) — `dismiss_overlay_result` command.
- `src-tauri/src/lib.rs` — register new command.
- `src-tauri/src/tray.rs`, `src-tauri/src/lib.rs` — window/tray title strings.
- `src/components/icons/index.ts` — export renamed components.
- `src/overlay/RecordingOverlay.tsx`, `src/overlay/RecordingOverlay.css` — `result` state + copy + auto-hide.
- `src/components/settings/SoundPicker.tsx` — add Lisper option.
- `src/App.css` — violet palette.
- `src/i18n/locales/en/translation.json` + other locale files — brand strings + new keys.
- All import sites of `HandyHand` / `HandyTextLogo`.

---

# Workstream A — Rebrand

## Task 1: Identity & ownership config

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json:2`
- Modify: `src-tauri/Cargo.toml`
- Create: `NOTICE`

**Interfaces:**
- Produces: bundle identifier `com.lisper.app`, product name `lisper`, cargo lib name `lisper_app_lib`, bin name `lisper`.

- [ ] **Step 1: Edit `tauri.conf.json`**

Set:
```json
"productName": "lisper",
"identifier": "com.lisper.app",
```
In `bundle.windows`, remove the `signCommand` line (leave the `nsis` block). In `plugins.updater`, disable updates for now by replacing the block with:
```json
"updater": {
  "active": false,
  "endpoints": ["https://example.invalid/latest.json"]
}
```
(Keep `createUpdaterArtifacts` as-is; `active: false` stops update checks. Removing `pubkey` is fine since updater is inactive.)

- [ ] **Step 2: Edit `package.json`**

Change `"name": "handy-app"` → `"name": "lisper-app"`.

- [ ] **Step 3: Edit `src-tauri/Cargo.toml`**

Rename the binary `name = "handy"` → `name = "lisper"` and the lib `name = "handy_app_lib"` → `name = "lisper_app_lib"`. Then grep for the old lib name and update references:

Run: `grep -rn "handy_app_lib" src-tauri/src`
Update each hit (commonly `main.rs` calling `handy_app_lib::run()`) to `lisper_app_lib`.

- [ ] **Step 4: Create `NOTICE`**

```
lisper

This product is a fork of Handy (https://github.com/cjpais/Handy),
Copyright (c) 2025 CJ Pais, licensed under the MIT License.
The full MIT License text is retained in the LICENSE file.

Modifications in this fork (branding, sounds, overlay features)
are likewise distributed under the MIT License.
```

- [ ] **Step 5: Verify backend compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors referencing `handy_app_lib`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml src-tauri/src NOTICE
git commit -m "chore: rebrand identity to lisper (bundle id, names, NOTICE)"
```

---

## Task 2: Robot logo source SVG + raster icon generation

**Files:**
- Create: `assets/branding/lisper-bot.svg`
- Create: `scripts/gen-icons.sh`
- Modify (regenerate): all files under `src-tauri/icons/` and the tray PNGs in `src-tauri/resources/` (`tray_idle.png`, `tray_idle_dark.png`, `tray_recording.png`, `tray_recording_dark.png`, `tray_transcribing.png`, `tray_transcribing_dark.png`, `handy.png`, `recording.png`, `transcribing.png`)

**Interfaces:**
- Produces: `assets/branding/lisper-bot.svg` (square viewBox, violet fill `#7C3AED`), and regenerated rasters in every icon slot referenced by `tauri.conf.json` `bundle.icon`.

- [ ] **Step 1: Create the source SVG**

Create `assets/branding/lisper-bot.svg` — a simple, legible robot head that reads at 16px. Example minimal mark (replace/refine as desired):

```svg
<svg width="512" height="512" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
  <rect width="512" height="512" rx="96" fill="#7C3AED"/>
  <!-- antenna -->
  <line x1="256" y1="96" x2="256" y2="150" stroke="#EDE9FE" stroke-width="14" stroke-linecap="round"/>
  <circle cx="256" cy="90" r="18" fill="#EDE9FE"/>
  <!-- head -->
  <rect x="136" y="150" width="240" height="200" rx="44" fill="#EDE9FE"/>
  <!-- eyes -->
  <circle cx="206" cy="240" r="26" fill="#7C3AED"/>
  <circle cx="306" cy="240" r="26" fill="#7C3AED"/>
  <!-- mouth -->
  <rect x="196" y="296" width="120" height="18" rx="9" fill="#7C3AED"/>
</svg>
```

- [ ] **Step 2: Write `scripts/gen-icons.sh`**

This generates every raster Tauri needs from the source SVG. macOS-first (uses `sips`/`iconutil`); falls back to ImageMagick if present.

```bash
#!/usr/bin/env bash
set -euo pipefail
SRC="assets/branding/lisper-bot.svg"
OUT="src-tauri/icons"
RES="src-tauri/resources"

render() { # render <size> <dest.png>
  if command -v rsvg-convert >/dev/null; then
    rsvg-convert -w "$1" -h "$1" "$SRC" -o "$2"
  elif command -v magick >/dev/null; then
    magick -background none -size "${1}x${1}" "$SRC" "$2"
  else
    echo "Need rsvg-convert or imagemagick (magick)"; exit 1
  fi
}

# Core PNGs
render 32   "$OUT/32x32.png"
render 128  "$OUT/128x128.png"
render 256  "$OUT/128x128@2x.png"
render 512  "$OUT/icon.png"
render 512  "$OUT/logo.png"
# Windows Store / Square logos
render 30   "$OUT/Square30x30Logo.png"
render 44   "$OUT/Square44x44Logo.png"
render 71   "$OUT/Square71x71Logo.png"
render 89   "$OUT/Square89x89Logo.png"
render 107  "$OUT/Square107x107Logo.png"
render 142  "$OUT/Square142x142Logo.png"
render 150  "$OUT/Square150x150Logo.png"
render 284  "$OUT/Square284x284Logo.png"
render 310  "$OUT/Square310x310Logo.png"
render 50   "$OUT/StoreLogo.png"

# macOS .icns via iconset
if command -v iconutil >/dev/null; then
  ICONSET="$(mktemp -d)/lisper.iconset"; mkdir -p "$ICONSET"
  for s in 16 32 128 256 512; do
    render "$s"           "$ICONSET/icon_${s}x${s}.png"
    render $((s*2))       "$ICONSET/icon_${s}x${s}@2x.png"
  done
  iconutil -c icns "$ICONSET" -o "$OUT/icon.icns"
fi

# .ico (multi-size) via magick if available
if command -v magick >/dev/null; then
  magick "$OUT/256.png" 2>/dev/null || true
  magick -background none "$SRC" -define icon:auto-resize=256,128,64,48,32,16 "$OUT/icon.ico"
fi

# Tray icons (monochrome-ish, reuse mark; light + dark variants share art here)
for f in tray_idle tray_idle_dark tray_recording tray_recording_dark tray_transcribing tray_transcribing_dark; do
  render 64 "$RES/$f.png"
done
render 512 "$RES/handy.png"
render 512 "$RES/recording.png"
render 512 "$RES/transcribing.png"
echo "icons generated"
```

(iOS `AppIcon-*` set under `src-tauri/icons/ios/` is only used for iOS builds, which are out of scope; leave as-is or regenerate later.)

- [ ] **Step 3: Run the generator**

Run: `chmod +x scripts/gen-icons.sh && ./scripts/gen-icons.sh`
Expected: prints `icons generated`; `src-tauri/icons/icon.png` etc. updated. If neither `rsvg-convert` nor `magick` is installed, install one (`brew install librsvg` or `brew install imagemagick`) and rerun.

- [ ] **Step 4: Verify Tauri can load icons (build sanity)**

Run: `cd src-tauri && cargo check`
Expected: no icon path errors. (Full visual check happens in Task 4's dev-run.)

- [ ] **Step 5: Commit**

```bash
git add assets/branding/lisper-bot.svg scripts/gen-icons.sh src-tauri/icons src-tauri/resources/*.png
git commit -m "feat: add lisper robot logo and regenerate all app/tray icons"
```

---

## Task 3: Logo React components + replace usages

**Files:**
- Create: `src/components/icons/LisperBot.tsx`
- Create: `src/components/icons/LisperTextLogo.tsx`
- Modify: `src/components/icons/index.ts`
- Delete: `src/components/icons/HandyHand.tsx`, `src/components/icons/HandyTextLogo.tsx`
- Modify: all importers (find in Step 4)

**Interfaces:**
- Consumes: nothing.
- Produces: `LisperBot` (props `{ width?, height? }`, default 126×135-ish, classes `fill-text stroke-text`) and `LisperTextLogo` (props `{ width?, height?, className? }`).

- [ ] **Step 1: Create `LisperBot.tsx`**

```tsx
const LisperBot = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 128}
    height={height || 128}
    viewBox="0 0 512 512"
    className="fill-text stroke-text"
    xmlns="http://www.w3.org/2000/svg"
  >
    <line x1="256" y1="96" x2="256" y2="150" strokeWidth="14" strokeLinecap="round" />
    <circle cx="256" cy="90" r="18" />
    <rect x="136" y="150" width="240" height="200" rx="44" className="fill-logo-primary" />
    <circle cx="206" cy="240" r="26" />
    <circle cx="306" cy="240" r="26" />
    <rect x="196" y="296" width="120" height="18" rx="9" />
  </svg>
);

export default LisperBot;
```

- [ ] **Step 2: Create `LisperTextLogo.tsx`**

A simple text-based wordmark (avoids hand-tracing glyph paths). Uses currentColor so existing logo CSS classes still apply:

```tsx
import React from "react";

const LisperTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => (
  <svg
    width={width}
    height={height}
    className={className}
    viewBox="0 0 360 100"
    xmlns="http://www.w3.org/2000/svg"
  >
    <text
      x="0"
      y="74"
      fontFamily="system-ui, -apple-system, sans-serif"
      fontSize="84"
      fontWeight="700"
      className="logo-primary"
      fill="currentColor"
    >
      lisper
    </text>
  </svg>
);

export default LisperTextLogo;
```

- [ ] **Step 3: Update `src/components/icons/index.ts`**

Add exports (keep existing three):
```ts
export { default as LisperBot } from "./LisperBot";
export { default as LisperTextLogo } from "./LisperTextLogo";
```

- [ ] **Step 4: Find and update all importers**

Run: `grep -rn "HandyHand\|HandyTextLogo" src`
For each hit, replace the import and JSX usage: `HandyHand` → `LisperBot`, `HandyTextLogo` → `LisperTextLogo`. Then delete the old files:
```bash
git rm src/components/icons/HandyHand.tsx src/components/icons/HandyTextLogo.tsx
```

- [ ] **Step 5: Typecheck + lint**

Run: `bun run build`
Expected: TypeScript build succeeds, no unresolved `HandyHand`/`HandyTextLogo` references.

- [ ] **Step 6: Commit**

```bash
git add src/components/icons package.json
git commit -m "feat: replace hand logo components with lisper robot + wordmark"
```

---

## Task 4: Violet color scheme

**Files:**
- Modify: `src/App.css:1-110` (the `:root` light block and `prefers-color-scheme: dark` block)
- Modify: `src/overlay/RecordingOverlay.css` (audit for hardcoded hex)

**Interfaces:**
- Produces: violet values for `--color-background-ui`, `--color-logo-primary`, `--color-logo-stroke`, dark-mode equivalents.

- [ ] **Step 1: Edit light-mode `:root` in `src/App.css`**

Replace the pink values:
```css
--color-background-ui: #7C3AED;
--color-logo-primary: #a78bfa;
--color-logo-stroke: #4c1d95;
```
(Leave `--color-text`, `--color-background`, `--color-text-stroke`, grays as-is unless they read pink.)

- [ ] **Step 2: Edit dark-mode block (`@media (prefers-color-scheme: dark)`)**

Replace:
```css
--color-logo-primary: #c4b5fd;
--color-logo-stroke: #ede9fe;
```

- [ ] **Step 3: Audit overlay CSS + stray hex**

Run: `grep -rn "#da5893\|#faa2ca\|#f28cbb\|#382731\|#FAA2CA" src`
Expected: no remaining pink brand hex outside commented-out blocks. Replace any live ones with the violet equivalents above.

- [ ] **Step 4: Visual verification (dev run)**

Run: `bun run tauri dev` (or `CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev` on macOS if cmake errors)
Expected: app launches as "lisper" in the title bar/dock, robot icon in tray, UI accent is violet, logo is the robot. Confirm both light and dark OS appearance. Close the app.

- [ ] **Step 5: Commit**

```bash
git add src/App.css src/overlay/RecordingOverlay.css
git commit -m "feat: recolor UI to violet (#7C3AED) theme"
```

---

## Task 5: Brand strings (i18n + Rust titles)

**Files:**
- Modify: `src/i18n/locales/en/translation.json`
- Modify: other `src/i18n/locales/*/translation.json` brand occurrences
- Modify: `src-tauri/src/tray.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/overlay.rs` (any window/tray title literal "Handy")

**Interfaces:**
- Produces: no "Handy" brand name in user-facing UI (adjective/URL/doc references excepted).

- [ ] **Step 1: Find brand-name string occurrences in en locale**

Run: `grep -n "Handy" src/i18n/locales/en/translation.json`
For each value where "Handy" is the **app name** (e.g. "Welcome to Handy", "About Handy"), change to "lisper". Leave any occurrence where "handy" is the adjective.

- [ ] **Step 2: Apply the same to other locales**

Run: `grep -rln "Handy" src/i18n/locales`
For each file, replace app-name occurrences with "lisper". (These are proper-noun brand mentions; the word is the same across languages.)

- [ ] **Step 3: Update Rust window/tray titles**

Run: `grep -rn '"Handy"' src-tauri/src`
Replace title/tooltip string literals "Handy" → "lisper" (window title in `overlay.rs`/`lib.rs`, tray tooltip in `tray.rs`). Do not change the `handy-{timestamp}.wav` recording filename in `actions.rs` (internal, not user-facing) unless desired.

- [ ] **Step 4: Verify**

Run: `bun run build && cd src-tauri && cargo check`
Expected: both succeed.

- [ ] **Step 5: Commit**

```bash
git add src/i18n src-tauri/src
git commit -m "feat: replace Handy brand strings with lisper in UI and titles"
```

---

# Workstream B — Deep click sound

## Task 6: Generate placeholder Lisper WAVs

**Files:**
- Create: `scripts/gen-sounds.ts`
- Create: `src-tauri/resources/lisper_start.wav`, `src-tauri/resources/lisper_stop.wav`

**Interfaces:**
- Produces: two 16-bit PCM mono WAV files at 44.1kHz.

- [ ] **Step 1: Write `scripts/gen-sounds.ts`**

Generates a deep, satisfying low-frequency click for start and a softer release tick for stop. Pure Bun/Node, no deps.

```ts
import { writeFileSync } from "fs";

const SR = 44100;

function wav(samples: Float32Array): Buffer {
  const n = samples.length;
  const buf = Buffer.alloc(44 + n * 2);
  buf.write("RIFF", 0); buf.writeUInt32LE(36 + n * 2, 4); buf.write("WAVE", 8);
  buf.write("fmt ", 12); buf.writeUInt32LE(16, 16); buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22); buf.writeUInt32LE(SR, 24); buf.writeUInt32LE(SR * 2, 28);
  buf.writeUInt16LE(2, 32); buf.writeUInt16LE(16, 34);
  buf.write("data", 36); buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    buf.writeInt16LE((s * 32767) | 0, 44 + i * 2);
  }
  return buf;
}

// Deep click: low sine (~120Hz) + short noise transient, fast exponential decay
function deepClick(durSec: number, freq: number): Float32Array {
  const n = Math.floor(SR * durSec);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const env = Math.exp(-t * 32);
    const body = Math.sin(2 * Math.PI * freq * t) * env;
    const transient = (Math.sin(i * 12.9898) * 43758.5453 % 1) * Math.exp(-t * 220) * 0.4;
    out[i] = (body * 0.8 + transient) * 0.9;
  }
  return out;
}

writeFileSync("src-tauri/resources/lisper_start.wav", wav(deepClick(0.18, 120)));
writeFileSync("src-tauri/resources/lisper_stop.wav", wav(deepClick(0.12, 90)));
console.log("sounds generated");
```

- [ ] **Step 2: Run it**

Run: `bun scripts/gen-sounds.ts`
Expected: prints `sounds generated`; both WAV files exist.

- [ ] **Step 3: Verify the files are valid WAVs**

Run: `file src-tauri/resources/lisper_start.wav src-tauri/resources/lisper_stop.wav`
Expected: both report `RIFF (little-endian) data, WAVE audio`.

- [ ] **Step 4: Commit**

```bash
git add scripts/gen-sounds.ts src-tauri/resources/lisper_start.wav src-tauri/resources/lisper_stop.wav
git commit -m "feat: add generated lisper deep-click start/stop sounds"
```

---

## Task 7: SoundTheme::Lisper enum + default + picker option

**Files:**
- Modify: `src-tauri/src/settings.rs` (enum at ~236, `as_str` at ~242, `default_sound_theme` at ~502)
- Modify: `src/components/settings/SoundPicker.tsx`
- Modify: `src/i18n/locales/en/translation.json` (label if SoundPicker uses a key)

**Interfaces:**
- Consumes: existing `SoundTheme` enum, `to_start_path`/`to_stop_path` (unchanged — they call `as_str()`).
- Produces: `SoundTheme::Lisper` (serde `"lisper"`), default theme = `Lisper`, frontend option `{ value: "lisper", label: "Lisper" }`.

- [ ] **Step 1 (test-first): add a Rust unit test for serde + path mapping**

Append to `src-tauri/src/settings.rs` (in or after the existing tests module; if none, add `#[cfg(test)] mod tests { ... }`):

```rust
#[cfg(test)]
mod sound_theme_tests {
    use super::SoundTheme;
    #[test]
    fn lisper_theme_paths_and_serde() {
        let t = SoundTheme::Lisper;
        assert_eq!(t.to_start_path(), "resources/lisper_start.wav");
        assert_eq!(t.to_stop_path(), "resources/lisper_stop.wav");
        assert_eq!(serde_json::to_string(&t).unwrap(), "\"lisper\"");
        assert_eq!(super::default_sound_theme(), SoundTheme::Lisper);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test sound_theme_tests`
Expected: FAIL — `SoundTheme` has no variant `Lisper`.

- [ ] **Step 3: Implement the enum + mapping + default**

In `settings.rs`:
```rust
pub enum SoundTheme {
    Lisper,
    Marimba,
    Pop,
    Custom,
}
```
In `as_str`, add the arm:
```rust
SoundTheme::Lisper => "lisper",
```
Change `default_sound_theme`:
```rust
fn default_sound_theme() -> SoundTheme {
    SoundTheme::Lisper
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd src-tauri && cargo test sound_theme_tests`
Expected: PASS.

- [ ] **Step 5: Add the frontend option**

In `src/components/settings/SoundPicker.tsx`, change the default and options:
```tsx
const selectedTheme = getSetting("sound_theme") ?? "lisper";

const options: DropdownOption[] = [
  { value: "lisper", label: "Lisper" },
  { value: "marimba", label: "Marimba" },
  { value: "pop", label: "Pop" },
];
```
And widen the cast in `onSelect`:
```tsx
updateSetting("sound_theme", value as "lisper" | "marimba" | "pop" | "custom")
```

- [ ] **Step 6: Regenerate bindings / typecheck**

Run: `bun run build`
Expected: succeeds. (The `bindings.ts` `SoundTheme` type is generated by tauri-specta at build of the Rust app; if the TS union is stale, run `bun run tauri dev` once to regenerate, or hand-add `"lisper"` to the union in `src/bindings.ts`.)

- [ ] **Step 7: Verify in app**

Run: `bun run tauri dev`. In Settings → sound theme, confirm "Lisper" is present, is the default, and the preview Play button plays the deep click. Close app.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/settings.rs src/components/settings/SoundPicker.tsx src/bindings.ts
git commit -m "feat: add Lisper sound theme as default deep-click feedback"
```

---

# Workstream C — Expandable copyable result panel

## Task 8: Backend — emit result event, resize/reset overlay, dismiss command

**Files:**
- Modify: `src-tauri/src/overlay.rs` (add `resize_overlay`, `reset_overlay_size`; reuse `OVERLAY_WIDTH`/`OVERLAY_HEIGHT`)
- Modify: `src-tauri/src/actions.rs` (non-empty `final_text` branch, ~lines 615-640)
- Modify: `src-tauri/src/commands/` overlay command module + `src-tauri/src/lib.rs` (register command)

**Interfaces:**
- Produces:
  - Event `transcription-result` → overlay window, payload `String` (final text).
  - `pub fn resize_overlay(app: &AppHandle, width: f64, height: f64)` and `pub fn reset_overlay_size(app: &AppHandle)` in `overlay.rs`.
  - Tauri command `dismiss_overlay_result(app: AppHandle)` that calls `reset_overlay_size` + `hide_recording_overlay`.
- Consumes: existing `hide_recording_overlay`, `OVERLAY_WIDTH`, `OVERLAY_HEIGHT`, the result-emitting site in `actions.rs`.

- [ ] **Step 1: Add resize/reset fns in `overlay.rs`**

After `hide_recording_overlay`, add:
```rust
const RESULT_WIDTH: f64 = 360.0;
const RESULT_HEIGHT: f64 = 120.0;

pub fn resize_overlay(app_handle: &AppHandle, width: f64, height: f64) {
    if let Some(win) = app_handle.get_webview_window("recording_overlay") {
        let _ = win.set_size(PhysicalSize::new(width as u32, height as u32));
    }
    update_overlay_position(app_handle);
}

pub fn reset_overlay_size(app_handle: &AppHandle) {
    if let Some(win) = app_handle.get_webview_window("recording_overlay") {
        let _ = win.set_size(PhysicalSize::new(
            OVERLAY_WIDTH as u32,
            OVERLAY_HEIGHT as u32,
        ));
    }
    update_overlay_position(app_handle);
}
```
(If the existing `update_overlay_position` assumes the bar height when centering, that is acceptable — the result panel still anchors to the same edge.)

- [ ] **Step 2: Add the dismiss command**

Find the overlay command module:

Run: `grep -rln "recording_overlay\|hide_recording_overlay\|tauri::command" src-tauri/src/commands`
In the overlay-related command file (create `src-tauri/src/commands/overlay.rs` and `mod overlay;` if none exists), add:
```rust
#[tauri::command]
#[specta::specta]
pub fn dismiss_overlay_result(app: tauri::AppHandle) {
    crate::overlay::reset_overlay_size(&app);
    crate::utils::hide_recording_overlay(&app);
}
```

- [ ] **Step 3: Register the command in `lib.rs`**

In the `tauri_specta`/`invoke_handler` command list in `src-tauri/src/lib.rs` (same list that holds `commands::audio::play_test_sound`), add:
```rust
commands::overlay::dismiss_overlay_result,
```

- [ ] **Step 4: Emit result + resize instead of immediate hide**

In `actions.rs`, in the `else` branch where `final_text` is non-empty (currently: paste, then `utils::hide_recording_overlay(&ah_clone)`), keep the paste but replace the unconditional hide with a result emission. After the paste call inside the `run_on_main_thread` closure, change:
```rust
                                    utils::hide_recording_overlay(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
```
to:
```rust
                                    crate::overlay::resize_overlay(&ah_clone, 360.0, 120.0);
                                    let _ = ah_clone.emit_to(
                                        "recording_overlay",
                                        "transcription-result",
                                        result_text_for_overlay.clone(),
                                    );
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
```
Before the `run_on_main_thread` call, clone the text for the overlay (since `final_text` is moved into `paste`):
```rust
                                let result_text_for_overlay = final_text.clone();
```
The fallback error arm of `run_on_main_thread` should still call `utils::hide_recording_overlay(&ah)` (unchanged). The overlay is now dismissed by the frontend via `dismiss_overlay_result`.

- [ ] **Step 5: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: compiles; `Emitter` trait already imported in `overlay.rs`/`actions.rs` (used by existing `.emit`). If `emit_to` needs the trait in `actions.rs`, it is already imported (existing `ah_clone.emit("paste-error", ())`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/overlay.rs src-tauri/src/actions.rs src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: emit transcription result to overlay and add resize/dismiss"
```

---

## Task 9: Frontend — result panel, copy, auto-hide-with-hover

**Files:**
- Modify: `src/overlay/RecordingOverlay.tsx`
- Modify: `src/overlay/RecordingOverlay.css`
- Modify: `src/i18n/locales/en/translation.json` (keys `overlay.copy`, `overlay.copied`)

**Interfaces:**
- Consumes: event `transcription-result` (string payload), command `commands.dismissOverlayResult()` (from regenerated `bindings.ts`), clipboard plugin `@tauri-apps/plugin-clipboard-manager`.
- Produces: overlay `"result"` state UI.

- [ ] **Step 1: Add i18n keys**

In `src/i18n/locales/en/translation.json`, under the existing `overlay` object add:
```json
"copy": "Copy",
"copied": "Copied"
```

- [ ] **Step 2: Extend overlay state + listeners in `RecordingOverlay.tsx`**

Change the state type and add result handling:
```tsx
type OverlayState = "recording" | "transcribing" | "processing" | "result";
```
Add state:
```tsx
const [resultText, setResultText] = useState("");
const [copied, setCopied] = useState(false);
const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```
Inside `setupEventListeners`, add a listener (and include it in the cleanup):
```tsx
const unlistenResult = await listen<string>("transcription-result", (event) => {
  setResultText(event.payload);
  setCopied(false);
  setState("result");
  setIsVisible(true);
});
```
Add `unlistenResult();` to the returned cleanup.

- [ ] **Step 3: Add auto-hide-with-hover timer + dismiss**

```tsx
import { commands } from "@/bindings";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const dismiss = () => {
  if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
  setIsVisible(false);
  setState("recording");
  commands.dismissOverlayResult();
};

const startHideTimer = () => {
  if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
  hideTimerRef.current = setTimeout(dismiss, 5000);
};

useEffect(() => {
  if (state === "result") startHideTimer();
  return () => {
    if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
  };
}, [state]);

const handleCopy = async () => {
  await writeText(resultText);
  setCopied(true);
};
```

- [ ] **Step 4: Render the result panel**

In the JSX, add a branch (e.g. in `overlay-middle`, plus pause-on-hover handlers on the root):
```tsx
<div
  ...
  onMouseEnter={() => {
    if (state === "result" && hideTimerRef.current) clearTimeout(hideTimerRef.current);
  }}
  onMouseLeave={() => {
    if (state === "result") startHideTimer();
  }}
>
```
And inside the middle region:
```tsx
{state === "result" && (
  <div className="result-panel">
    <div className="result-text">{resultText}</div>
    <button className="result-copy" onClick={handleCopy}>
      {copied ? t("overlay.copied") : t("overlay.copy")}
    </button>
  </div>
)}
```

- [ ] **Step 5: Style the panel in `RecordingOverlay.css`**

Add (violet accent, scrollable text, copy button):
```css
.result-panel {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 0 8px;
}
.result-text {
  flex: 1;
  max-height: 96px;
  overflow-y: auto;
  font-size: 12px;
  line-height: 1.3;
  text-align: left;
}
.result-copy {
  flex-shrink: 0;
  background: var(--color-background-ui);
  color: white;
  border: none;
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
}
```

- [ ] **Step 6: Build + verify end-to-end**

Run: `bun run tauri dev`. Trigger a transcription. Expected: bar expands into the panel showing the text; clicking **Copy** shows "Copied" and the clipboard holds the text; panel auto-hides ~5s after you stop hovering; hovering keeps it open. Close app.

- [ ] **Step 7: Commit**

```bash
git add src/overlay/RecordingOverlay.tsx src/overlay/RecordingOverlay.css src/i18n/locales/en/translation.json src/bindings.ts
git commit -m "feat: expand overlay into copyable result panel with hover-aware auto-hide"
```

---

# Final verification

- [ ] **Format check:** `bun run format && bun run lint`
- [ ] **Frontend build:** `bun run build` → succeeds.
- [ ] **Backend tests:** `cd src-tauri && cargo test` → passes (incl. `sound_theme_tests`).
- [ ] **Full app run:** `bun run tauri dev` → app is "lisper", violet, robot icons everywhere (window, tray), deep-click on mic activation, and dictation produces the copyable auto-hiding panel.
- [ ] **License check:** `LICENSE` (MIT, CJ Pais) intact and `NOTICE` present.

---

## Self-review notes (author)

- **Spec §1 license** → Task 1 (NOTICE) + Final verification. ✓
- **Spec §2.A1 identity** → Task 1. ✓
- **Spec §2.A2 strings** → Task 5. ✓
- **Spec §2.A3 code identifiers** → Task 3. ✓
- **Spec §2.A4 robot logo + all icon slots** → Tasks 2, 3. ✓
- **Spec §2.A5 violet palette** → Task 4. ✓
- **Spec §3 deep-click default sound** → Tasks 6, 7. ✓
- **Spec §4 expandable copyable panel, auto-hide+hover** → Tasks 8, 9. ✓
- **Spec §6 out of scope** honored: updater disabled (not hosted), no signing, English-only new keys. ✓
- **Naming consistency:** `dismiss_overlay_result` (Rust) ↔ `commands.dismissOverlayResult()` (TS, tauri-specta camelCases); `transcription-result` event name identical in `actions.rs` emit and `RecordingOverlay.tsx` listen; `resize_overlay`/`reset_overlay_size` used consistently in Tasks 8. ✓
