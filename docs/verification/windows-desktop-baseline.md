# Windows notype desktop baseline (#9)

Verified on the development Windows x64 machine, 2026-10-25. This is the formal Rust/Tauri/React baseline, separate from the Node diagnostic prototype.

## Source and policy

`desktop/` contains Handy v0.9.8 source at `14f6f0d31cb22a4268acfbddba3052820fd0a06e`, from archive SHA256 `a6bcd28cfb83dee566328432ce1d15e2a4f881d0dee8af8ca7a4d2f5499488f6`. Original MIT LICENSE and archived upstream agent instructions are retained. Dependencies and compiled outputs are excluded. All four feedback WAVs, the 1,807,522-byte Silero ONNX resource and its vocabulary resource are tracked.

Product/window identity is notype / `io.github.swchen44.notype`; default paths come from the platform per-user app resolver. `NOTYPE_DATA_DIR` is an absolute external override, with relative overrides rejected before initialization. Updater plugin, permission and controls are disabled, stored updater/auto-submit flags are constrained on read/write, enabling either policy is rejected through IPC, and the central return-key emitter rejects submission. External-script paste is blocked. Native DLL staging is retained.

## Executed checks

| Check | Result |
| --- | --- |
| `bun install --cwd desktop --frozen-lockfile` | Passed |
| `bun run --cwd desktop build` | TypeScript + Vite passed, 2,115 modules; existing chunk-size warning |
| `bun run --cwd desktop lint` | Passed |
| Prettier on changed frontend/config files | Passed |
| `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check` | Passed |
| `node scripts/test-desktop-policy.mjs` | 3 unit tests passed, 0 failed/ignored |
| `bun run --cwd desktop tauri build --no-bundle --debug` | Passed, Windows x64 CPU backend; 12 transcribe/ggml DLLs staged |
| `NOTYPE_DESKTOP_EXE=... node --test tests/integration/desktop-settings.runtime.mjs` | 1 actual native integration test passed, 0 skipped; recorded runs 4.28–5.41 s |

The native receipt launches the real executable twice with fresh external settings and all shortcuts unbound. It observes the notype window title and onboarding logo, reads the actual data root, constrains poisoned legacy policy flags, rejects both enable commands and the absent updater plugin, changes UI language through IPC and reads `zh-TW` back after restart. Microphone capture, paste, automatic Enter and live API calls are never invoked. Temporary test data is removed; default user settings are untouched.

## Red/green and native diagnosis

The data-root seam first failed with the missing product module, then passed after the platform path policy was added. The runtime test fails explicitly when its executable is unavailable; no missing prerequisite is counted as a skip.

The initial native build failed in upstream `transcribe-cpp-sys 0.2.4` Canary `model.cpp:626` with MSVC C2065 (`sp_space_id` undeclared), followed by parser errors. A seven-line UTF-8/LF/no-BOM compiler probe using the same trailing U+2581 comment failed under CP950 (exit 2) and passed with `/utf-8` (exit 0). Repository-local `desktop/.cargo/config.toml` supplies MSVC `_CL_=/utf-8`; the original native build then passed. Neither the global Cargo configuration nor registry source cache was patched.

The elevated tool process also caused WebView2 to ignore its environment browser flags. A Windows debug-only, numeric `NOTYPE_DEBUG_WEBVIEW_PORT` option enables the actual IPC receipt; release builds omit it. This behavior follows [Microsoft's elevated WebView2 override documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security#for-an-elevated-host-app-use-appropriate-override-flags). No global environment or registry override is installed.

## Limits

Only the Windows x64 debug no-bundle build and isolated settings receipt are verified here. Release payload completeness, other machines, macOS/Linux adapters, STT quality, session orchestration, original-target delivery and company/five-app acceptance remain their own tickets. No installer was built or launched, and Vulkan installation was not retried. The local static ORT build leaves a zero-byte optional `DirectML.dll` in the shared target; payload assembly must validate files rather than copying that artifact. Existing upstream Rust warnings remain in secure-input/transcription code; source import also retains upstream generated-bindings/Swift whitespace. No API key is needed for these checks.
