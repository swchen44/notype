# notype desktop development

The application source is vendored here, with the fixed upstream archive and MIT license recorded in [UPSTREAM.md](UPSTREAM.md). Dependency installation requires Bun; native Windows builds require Rust MSVC, Visual Studio C++ Build Tools, a Windows SDK and CMake. Windows x64 builds use CPU backends and do not require Vulkan.

From the repository root:

```powershell
bun install --cwd desktop --frozen-lockfile
bun run --cwd desktop build
bun run --cwd desktop lint
node scripts/test-desktop-policy.mjs

# Set build paths in the current process; never in global Cargo configuration.
$env:CARGO_TARGET_DIR = 'D:/git/notype-build'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
bun run --cwd desktop tauri build --no-bundle --debug

$env:NOTYPE_DESKTOP_EXE = 'D:/git/notype-build/debug/notype.exe'
node --test tests/integration/desktop-settings.runtime.mjs
```

Ensure Cargo and CMake are on PATH. The checked-in `.cargo/config.toml` makes MSVC read UTF-8 source; this is required for upstream C++ comments on a Windows system using code page 950. It does not change the global compiler configuration.

The default app identity is `io.github.swchen44.notype`. Settings, logs, history and models use platform per-user locations outside the npm program tree. An absolute `NOTYPE_DATA_DIR` selects an isolated directory for runtime verification; relative values fail before app initialization. The runtime test creates a fresh external directory, keeps every shortcut unbound and the microphone on demand, reads/changes settings over the actual WebView IPC, checks restart persistence, and removes its own temporary data. It does not record audio or invoke paste.

On Windows debug builds, the test selects a temporary CDP port through `NOTYPE_DEBUG_WEBVIEW_PORT`. Only a nonzero numeric port is accepted; release builds omit this hook. Explicit WebView2 options are needed because an elevated host ignores environment browser flags ([Microsoft documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security#for-an-elevated-host-app-use-appropriate-override-flags)).

The backend rejects auto-submit and installer-updater enablement even if a preexisting settings file enables them. Updater plugin permissions and UI controls are removed; product updates belong to the npm payload flow. `--no-bundle` is required: an installer is not a notype distribution artifact. Existing native DLL staging is retained, including transcribe/ggml backends, ONNX and VC runtimes.

This baseline does not claim complete npm payload packaging, company API access, full session integration or delivery acceptance in the five target applications. Future platform code remains available for macOS/Linux adapters; only Windows amd64 is currently validated.
