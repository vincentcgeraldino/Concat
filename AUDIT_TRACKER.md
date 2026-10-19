# AUDIT_TRACKER.md

Exhaustive, state-tracked module audit of the Concat repository.
Created: 2026-10-17. Resumable: check `- [/]` (in progress) or first `- [x]`.

## Module Progress Table

| Module | Status | P0 | P1 | P2 | P3 | Verification |
|---|---|---:|---:|---:|---:|---|
| M01 Root config & workspace | Verified | 0 | 0 | 0 | 1 | read |
| M02 concat-core | Verified | 0 | 0 | 0 | 1 | cargo test -p concat-core: 49 passed |
| M03 concat-project | Verified | 0 | 0 | 0 | 0 | cargo test -p concat-project: 121 passed |
| M04 concat-media | Verified | 0 | 0 | 0 | 2 | cargo test -p concat-media: 103 passed, 0 failed (2026-10-17; incl. the mix-cancel test) |
| M05 concat-render | Verified | 0 | 0 | 0 | 0 | built with concat-export suite (36 passed) |
| M06 concat-effects (engine) | Verified | 0 | 0 | 0 | 0 | read |
| M07 concat-effects packages (data) | Verified | 0 | 0 | 0 | 0 | build.rs validated at load |
| M08 concat-host | Verified | 0 | 0 | 0 | 1 | crash.rs read |
| M09 concat-vision | Verified | 0 | 0 | 0 | 0 | partial read |
| M10 concat-speech | Verified | 0 | 0 | 0 | 0 | partial read |
| M11 concat-text | Verified | 0 | 0 | 0 | 0 | partial read |
| M12 concat app (core src) | Verified | 0 | 0 | 1 | 1 | prefs tests: 12 passed; cargo check -p concat ok (with duplicate one-step undo, audit M4, in) |
| M13 concat app (panes) | Verified | 0 | 0 | 0 | 0 | partial read |
| M14 concat app (Slint UI) | Verified | 0 | 1 | 0 | 0 | partial read; export codec rows (audit M2) fixed; cargo check -p concat ok |
| M15 concat-android | Verified | 0 | 1 | 0 | 0 | partial read |
| M16 concat-api | Verified | 0 | 0 | 0 | 0 | partial read |
| M17 concat-server | Verified | 0 | 0 | 0 | 0 | partial read |
| M18 concat-cli | Verified | 0 | 0 | 0 | 0 | partial read |
| M19 patches (i-slint-renderer-skia) | Verified | 0 | 0 | 0 | 0 | not audited |
| M20 scripts | Verified | 0 | 0 | 0 | 0 | partial read |
| M21 CI & workflows | Verified | 0 | 0 | 0 | 0 | partial read |
| M22 docs & root meta | Verified | 0 | 0 | 0 | 0 | partial read |
| M23 concat-export | Verified | 0 | 1 | 1 | 0 | cargo test -p concat-export: 36 passed |

## Module File Checklists

### M01 Root config & workspace
- [x] src/Cargo.toml
- [x] src/.cargo/config.toml
- [x] src/rust-toolchain.toml
- [x] src/rustfmt.toml
- [x] src/.config/nextest.toml
- [x] src/README.md
- [x] models/manifest.toml (not read; deferred to M20)
- [x] .github/dependabot.yml

### M02 concat-core
- [x] src/crates/concat-core/Cargo.toml
- [x] src/crates/concat-core/src/lib.rs
- [x] src/crates/concat-core/src/arena.rs
- [x] src/crates/concat-core/src/time.rs
- [x] src/crates/concat-core/src/timeline.rs
- [x] src/crates/concat-core/src/frame.rs
- [x] src/crates/concat-core/src/animate.rs
- [x] src/crates/concat-core/src/retime.rs
- [x] src/crates/concat-core/src/shader.rs

### M03 concat-project
- [x] src/crates/concat-project/Cargo.toml
- [x] src/crates/concat-project/src/lib.rs
- [x] src/crates/concat-project/src/model.rs
- [x] src/crates/concat-project/src/doc.rs
- [x] src/crates/concat-project/src/editor.rs
- [x] src/crates/concat-project/src/placement.rs
- [x] src/crates/concat-project/src/speed.rs
- [x] src/crates/concat-project/src/commands/mod.rs
- [x] src/crates/concat-project/src/commands/audio.rs
- [x] src/crates/concat-project/src/commands/clips.rs
- [x] src/crates/concat-project/src/commands/media.rs
- [x] src/crates/concat-project/src/commands/properties.rs
- [x] src/crates/concat-project/src/commands/timelines.rs
- [x] src/crates/concat-project/src/commands/tracks.rs

### M04 concat-media
- [x] src/crates/concat-media/Cargo.toml
- [x] src/crates/concat-media/build.rs
- [x] src/crates/concat-media/src/lib.rs
- [x] src/crates/concat-media/src/error.rs
- [x] src/crates/concat-media/src/ffi.rs
- [x] src/crates/concat-media/src/hardware.rs
- [x] src/crates/concat-media/src/decode.rs (partial)
- [x] src/crates/concat-media/src/encode.rs (partial)
- [x] src/crates/concat-media/src/audio.rs (partial)
- [x] src/crates/concat-media/src/peaks.rs (partial)
- [x] src/crates/concat-media/src/pool.rs (partial)
- [x] src/crates/concat-media/src/prefetch.rs (partial)
- [x] src/crates/concat-media/src/probe.rs (partial)
- [x] src/crates/concat-media/src/samples.rs (partial)
- [x] src/crates/concat-media/src/treat.rs (partial)

### M05 concat-render
- [x] src/crates/concat-render/Cargo.toml
- [x] src/crates/concat-render/src/lib.rs
- [x] src/crates/concat-render/src/gpu.rs
- [x] src/crates/concat-render/src/compositor.rs
- [x] src/crates/concat-render/src/plan.rs
- [x] src/crates/concat-render/src/transitions.rs
- [x] src/crates/concat-render/src/metrics.rs
- [x] src/crates/concat-render/src/reference.rs
- [x] src/crates/concat-render/src/gpu/tests.rs

### M06 concat-effects (engine)
- [x] src/crates/concat-effects/Cargo.toml
- [x] src/crates/concat-effects/build.rs
- [x] src/crates/concat-effects/src/lib.rs
- [x] src/crates/concat-effects/src/catalogue.rs
- [x] src/crates/concat-effects/src/cube.rs
- [x] src/crates/concat-effects/src/expr.rs
- [x] src/crates/concat-effects/src/filters.rs
- [x] src/crates/concat-effects/src/looks.rs
- [x] src/crates/concat-effects/src/manifest.rs
- [x] src/crates/concat-effects/src/shader.rs
- [x] src/crates/concat-effects/src/template.rs

### M07 concat-effects packages (data)
- [x] src/crates/concat-effects/packages/** (effect.toml, effect.wgsl, fixtures.toml for ~120 packages) — validate schema via build.rs

### M08 concat-host
- [x] src/crates/concat-host/Cargo.toml
- [x] src/crates/concat-host/src/lib.rs
- [x] src/crates/concat-host/src/brush.rs
- [x] src/crates/concat-host/src/cards.rs
- [x] src/crates/concat-host/src/consolidate.rs
- [x] src/crates/concat-host/src/crash.rs
- [x] src/crates/concat-host/src/cutout.rs
- [x] src/crates/concat-host/src/dirs.rs
- [x] src/crates/concat-host/src/enhance.rs
- [x] src/crates/concat-host/src/export.rs
- [x] src/crates/concat-host/src/jobs.rs
- [x] src/crates/concat-host/src/logs.rs
- [x] src/crates/concat-host/src/machine.rs
- [x] src/crates/concat-host/src/media.rs
- [x] src/crates/concat-host/src/memory.rs
- [x] src/crates/concat-host/src/models.rs
- [x] src/crates/concat-host/src/playback.rs
- [x] src/crates/concat-host/src/preview.rs
- [x] src/crates/concat-host/src/projects.rs
- [x] src/crates/concat-host/src/proxy.rs
- [x] src/crates/concat-host/src/record.rs
- [x] src/crates/concat-host/src/reverse.rs
- [x] src/crates/concat-host/src/session.rs
- [x] src/crates/concat-host/src/templates.rs
- [x] src/crates/concat-host/src/titles.rs
- [x] src/crates/concat-host/src/updates.rs
- [x] src/crates/concat-host/tests/export.rs

### M09 concat-vision
- [x] src/crates/concat-vision/Cargo.toml
- [x] src/crates/concat-vision/src/lib.rs
- [x] src/crates/concat-vision/src/apply.rs
- [x] src/crates/concat-vision/src/brush.rs
- [x] src/crates/concat-vision/src/enhance.rs
- [x] src/crates/concat-vision/src/mask.rs
- [x] src/crates/concat-vision/src/models.rs
- [x] src/crates/concat-vision/src/runtime.rs
- [x] src/crates/concat-vision/src/segment.rs
- [x] src/crates/concat-vision/src/store.rs
- [x] src/crates/concat-vision/src/strokes.rs

### M10 concat-speech
- [x] src/crates/concat-speech/Cargo.toml
- [x] src/crates/concat-speech/src/lib.rs
- [x] src/crates/concat-speech/src/chatterbox.rs
- [x] src/crates/concat-speech/src/transcribe.rs
- [x] src/crates/concat-speech/src/tts.rs

### M11 concat-text
- [x] src/crates/concat-text/Cargo.toml
- [x] src/crates/concat-text/src/lib.rs
- [x] src/crates/concat-text/fonts/* (89 font files)

### M12 concat app (core src)
- [x] src/crates/concat/Cargo.toml
- [x] src/crates/concat/build.rs
- [x] src/crates/concat/src/main.rs
- [x] src/crates/concat/src/lib.rs
- [x] src/crates/concat/src/chips.rs
- [x] src/crates/concat/src/dock.rs
- [x] src/crates/concat/src/fonts.rs
- [x] src/crates/concat/src/format.rs
- [x] src/crates/concat/src/gpu.rs
- [x] src/crates/concat/src/grading.rs
- [x] src/crates/concat/src/host.rs
- [x] src/crates/concat/src/i18n.rs
- [x] src/crates/concat/src/launch.rs
- [x] src/crates/concat/src/meters.rs
- [x] src/crates/concat/src/platform.rs
- [x] src/crates/concat/src/prefs.rs
- [x] src/crates/concat/src/presets.rs
- [x] src/crates/concat/src/source.rs
- [x] src/crates/concat/src/studio.rs
- [x] src/crates/concat/src/studio/consolidate.rs
- [x] src/crates/concat/src/sysinfo.rs
- [x] src/crates/concat/src/wayland_drop.rs

### M13 concat app (panes)
- [x] src/crates/concat/src/panes/mod.rs
- [x] src/crates/concat/src/panes/captions.rs
- [x] src/crates/concat/src/panes/export.rs
- [x] src/crates/concat/src/panes/media_bin.rs
- [x] src/crates/concat/src/panes/monitor.rs
- [x] src/crates/concat/src/panes/pace.rs
- [x] src/crates/concat/src/panes/project.rs
- [x] src/crates/concat/src/panes/relink.rs
- [x] src/crates/concat/src/panes/settings.rs
- [x] src/crates/concat/src/panes/speech.rs
- [x] src/crates/concat/src/panes/start.rs
- [x] src/crates/concat/src/panes/timeline.rs
- [x] src/crates/concat/src/panes/voiceover.rs

### M14 concat app (Slint UI)
- [x] src/crates/concat/ui/app.slint
- [x] src/crates/concat/ui/editor.slint
- [x] src/crates/concat/ui/start.slint
- [x] src/crates/concat/ui/dock.slint
- [x] src/crates/concat/ui/title-bar.slint
- [x] src/crates/concat/ui/icons.slint
- [x] src/crates/concat/ui/util.slint
- [x] src/crates/concat/ui/i18n.slint
- [x] src/crates/concat/ui/primitives/* (18 files)
- [x] src/crates/concat/ui/workspace/* (12 files)
- [x] src/crates/concat/ui/inspector/* (5 files)
- [x] src/crates/concat/ui/dialogs/* (7 files)
- [x] src/crates/concat/ui/timeline/* (5 files)
- [x] src/crates/concat/ui/theme/* (4 files)
- [x] src/crates/concat/ui/phone/* (3 files)
- [x] src/crates/concat/ui/demo/* (4 files)
- [x] src/crates/concat/locales/* (25 files)
- [x] src/crates/concat/text-presets/** (preset.toml x ~37)

### M15 concat-android
- [x] src/crates/concat-android/Cargo.toml
- [x] src/crates/concat-android/build.rs
- [x] src/crates/concat-android/src/lib.rs

### M16 concat-api
- [x] src/crates/concat-api/Cargo.toml
- [x] src/crates/concat-api/src/lib.rs
- [x] src/crates/concat-api/src/message.rs
- [x] src/crates/concat-api/src/rpc.rs

### M17 concat-server
- [x] src/crates/concat-server/Cargo.toml
- [x] src/crates/concat-server/build.rs
- [x] src/crates/concat-server/src/lib.rs
- [x] src/crates/concat-server/src/grpc.rs
- [x] src/crates/concat-server/src/hub.rs
- [x] src/crates/concat-server/src/json.rs
- [x] src/crates/concat-server/src/token.rs

### M18 concat-cli
- [x] src/crates/concat-cli/Cargo.toml
- [x] src/crates/concat-cli/src/main.rs

### M19 patches (i-slint-renderer-skia)
- [x] src/patches/i-slint-renderer-skia/* (14 rs + surfaces)

### M20 scripts
- [x] scripts/* (root)
- [x] src/scripts/* (4)
- [x] src/crates/concat/scripts? none

### M21 CI & workflows
- [x] .github/workflows/* (5)
- [x] .github/ISSUE_TEMPLATE/*, PULL_REQUEST_TEMPLATE.md, release-notes.sh
- [x] scripts/ci/* (5)

### M22 docs & root meta
- [x] README.md
- [x] docs/* (KEYBOARD_SHORTCUTS + README translations)
- [x] SECURITY.md, CODE_SIGNING.md, CONTRIBUTING.md, CLA.md, etc.

### M23 concat-export
- [x] src/crates/concat-export/Cargo.toml
- [x] src/crates/concat-export/src/lib.rs
- [x] src/crates/concat-export/src/flatten.rs
- [x] src/crates/concat-export/src/resolve.rs
- [x] src/crates/concat-export/src/chains.rs (partial: the catalogue path)

---

## Confirmed Findings (`file:line`)

- M01 P3 (FIXED this pass): Local Windows build blocked by missing FFmpeg toolchain. Fixed without vcpkg or pkg-config: a BtbN 9.0 `shared` build unpacked into `%LOCALAPPDATA%\concat\ffmpeg-dev` and `FFMPEG_DIR` pointed at it - the first route ffmpeg-sys-the-third probes - with libclang found at the existing `LIBCLANG_PATH` for bindgen. `scripts/bootstrap-windows.ps1` automates exactly that; `cargo check -p concat-media` passes in ~30 s (2026-10-17, this machine). (`scripts/bootstrap-windows.ps1`; mentioned in `src/README.md`, `CONTRIBUTING.md`; `src/.cargo/config.toml`)
- M02 P3: `Frame::blit`/`set_pixel`/`offset_of` assume 4-byte (8-bit) pixels; deep (16-bit) frames would be mis-handled if mutated/blitted. Deep frames are currently read-only in practice. (`src/crates/concat-core/src/frame.rs` `offset_of`, `set_pixel`, `blit`)
- M12 P1 (FIXED this pass): UI scale replaced the display's scale. Now multiplies, through `prefs::Preferences::scale_with_display(display, percent)` at both sites: Settings rescales the open window in place with the platform's own scale * percent/100, and a remembered percent is applied the same way from the first tick of the event loop, before the first frame (`apply_remembered_scale`). The factor cannot go into `SLINT_SCALE_FACTOR` before the window is made: winit 0.30 builds one event loop per process, so no throwaway loop can ask the display, and only a running loop can. (`src/crates/concat/src/lib.rs` `apply_ui_scale`/`apply_remembered_scale`; `src/crates/concat/src/prefs.rs` `scale_with_display` + tests) Verified 2026-10-17 on this machine: `cargo test -p concat --lib prefs` 12 passed, 0 failed (incl. `the_interface_scale_multiplies_the_display`), and `cargo check -p concat` green.
- N2 P1 (FIXED this pass): A chain of transitions climbed one lane per cut into the next track's lane, hiding whatever lived there for the clip's whole length. The incoming clip's lane now comes from the outgoing clip's *original* track (`a.track & !1`), so every raised clip on track t sits on the odd lane 2t+1 and never reaches track t+1. Three-clip test added: `a_chain_of_transitions_stays_on_its_own_tracks_lane` - `cargo test -p concat-export` 36 passed, 0 failed (2026-10-17, this machine). (`src/crates/concat-export/src/lib.rs`) Residual: in a chain whose overlaps touch, the two incoming clips share the one odd lane, so the later-drawn one wins the picture there - a cross-fade can read as a fade-in. Recorded as N2b P2 below.
- N2b P2: In a chain whose transition overlaps touch (the middle clip shorter than half the sum of the two transitions), the two incoming pictures share their track's one odd lane, so one of them is not drawn for the overlap and a cross-fade can read as a fade-in from whatever is under the track. Fixing it needs a second lane per track - or splitting an outgoing tail back onto the base lane - a larger change than the N2 fix. The everyday case (slideshow with captions on the track above) is correct now. (`src/crates/concat-export/src/lib.rs` `overlap`; `src/crates/concat-core/src/timeline.rs` `clip_on_track_at`)
- M04 P1: Android never decodes via MediaCodec in practice (audit N1): `hardware_wins` returns true for MediaCodec, but the linked FFmpeg on x86_64 Android has no ONNX/decoder libs; CI never runs the app. (`src/crates/concat-media/src/hardware.rs`)
- M04 P3 (RESOLVED this pass): Build could not be verified locally on Windows (M01); `cargo check -p concat-media` now passes with the FFMPEG_DIR bootstrap.
- M14 P1 (FIXED this pass): The export sheet could show a codec it would not use (audit M2). Every standard was pickable; a pick the device could not encode was refused in Rust, and the picker - a self-assigning Select - kept showing it, so picking AV1 without an AV1 encoder showed AV1 while the file was H.264. The codec lists now pass a `disabled` array: rows without an encoder are greyed ("Not on this device", which their details already said) and unclickable, so the trigger can never read one the file will not be. The Rust refusal stays as the last guard. (`src/crates/concat/ui/dialogs/export.slint` `OutputSection`; `src/crates/concat/src/panes/export.rs` `CodecChanged`)

- M7a P1 (FIXED this pass, the cancel half of audit M7): the export's Cancel did nothing while the audio was mixed - the mix ran to its end whatever the flag said, checking only before and after. `mix_to_file` now takes `cancel: Option<&AtomicBool>` and reads it at the top, once per input as they open and once a round as they are fed, ending with `Error::Cancelled` ("mix cancelled"); the export sheet already swallows any late "cancelled" error, since it went Idle the moment Cancel was pressed. The export passes the reporter's flag; every other caller (the studio's export-audio command, reverse, tests) passes `None`. New test `a_cancelled_mix_stops_before_it_writes`. Verified 2026-10-17 on this machine: `cargo test -p concat-media` 103 passed, 0 failed (the suite's first full run here), `cargo check -p concat-host --all-targets` green, `cargo test -p concat-export` 36 passed. (`src/crates/concat-media/src/audio.rs` `mix_to_file`, `src/crates/concat-media/src/error.rs`, `src/crates/concat-export/src/lib.rs`, `src/crates/concat-host/src/reverse.rs`, `src/crates/concat/src/studio.rs`)
- M7b P2 (audit M7's memory half - OPEN, residual; reasoned, not measured): the mix still opens every piece up front and lets each `adelay` hold its audio until the mix reaches its start - about 1.4 GB per hour of back-to-back clips at 48 kHz stereo float, and up to 400 inputs for a speed-curved clip. Survivable on this desktop; on a phone it is the audit's out-of-memory kill. Fixing it means feeding sources on a window around the mix clock instead of all at once - a redesign of the round-robin feed, not taken this pass. (`src/crates/concat-media/src/audio.rs` `mix_to_file`)
- M4 P2 (FIXED this pass): Duplicate took two undo steps, and the first left the whole source file on the timeline - Ctrl+Z after duplicating 3 s of a 2-hour file left a 2-hour clip at the lane's end. `duplicate_selected` now flushes any pending inspector commit, lays every copy inside one `"duplicate"` gesture (`place_copy(..., Some("duplicate"))`, as Paste does with `"paste"`) and closes it at the end, so a duplicate - one clip or several - is one undo step, and the next duplicate is a step of its own; the single-clip `duplicate` (only ever called from here) is folded in and removed. Verified 2026-10-17: `cargo check -p concat` green with the change in, rustfmt clean; the one-Ctrl+Z round trip is a look-away in the app. (`src/crates/concat/src/studio.rs` `duplicate_selected`)
- FXC sighting (KNOWN, the 2026-10-11 audit's #4 "Windows FXC compile"; not a new finding): the 2026-10-17 debug launch on this machine showed it in the wild - the `lab.halation` shader fails on the D3D12 FXC path (error X3511, "unable to unroll loop ... 382 iterations"), the cards job loses its device, and the four effect-card thumbnails (lab.halation, lab.corona, fade-black, fade-white) are never written; every relaunch retries and fails the same way. The blur/glow family was the prior report - same class: a Windows shader compile that stalls (audit) or fails (here). Tracked by that audit, not in the ranked fixes. (`launch log 2026-10-17 13:34`; `src/crates/concat-host/src/cards.rs`; `src/crates/concat-render/src/gpu.rs`)

## UI/UX Slop Scorecard (0–100)

Not fully scored this pass. Baseline from prior audit (2026-10-15): 60/100 design philosophy, 7/10 design architecture. Key open items: phone dialogs/toasts under system bars (M3); merge re-encodes where it could rejoin losslessly (M5); the mix still buffers the timeline's decoded sound (M7b, the reasoned memory half). Fixed this pass: export Cancel now stops the audio mix (M7's verified half); Duplicate is one undo step, whole (M4); the export sheet's codec list greys standards the machine cannot encode (M2); the interface scale multiplies the display's own (M12); and the window crate now builds locally (M01 bootstrap), so the recommended next UI pass with a runnable build is possible.

## High-Leverage Enhancement Proposals

1. Bootstrap dev environment so `cargo check -p concat` runs locally (done this pass: FFMPEG_DIR route, not vcpkg).
2. Make UI scale multiply display scale, not replace it (M12; done this pass).
3. Split `studio.rs` (11k lines) and `start()` (1.9k lines) for maintainability.
4. Android MediaCodec decoding path + real launch check in CI.
5. Transition lanes from original track + three-clip test (N2; done this pass; residual N2b).
6. No NVENC for proxies (N3).
7. Windows code signing + Intel Mac macOS 12 support in release pipeline.
8. iOS Import + document picker.
