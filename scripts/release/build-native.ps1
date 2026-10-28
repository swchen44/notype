param(
    [Parameter(Mandatory = $true)][string]$SourceRevision,
    [Parameter(Mandatory = $true)][string]$OutputRoot
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $root
if ($env:RUNNER_OS -cne 'Windows' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted' -or
    -not $env:RUNNER_TEMP -or $SourceRevision -cne $env:GITHUB_SHA) { throw 'native-runner-invalid' }
$temp = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
$expectedOutput = [IO.Path]::GetFullPath((Join-Path $temp 'notype-native'))
$output = [IO.Path]::GetFullPath($OutputRoot)
if (-not [string]::Equals($output, $expectedOutput, [StringComparison]::OrdinalIgnoreCase)) { throw 'native-output-invalid' }
$buildDir = Join-Path $temp 'notype-native-build'
$cargoHome = Join-Path $temp 'notype-native-cargo-home'
$ortCache = Join-Path $temp 'notype-native-ort-cache'
$configFile = Join-Path $temp 'notype-native-build-input.json'
$tauriConfig = Join-Path $temp 'notype-native-tauri-config.json'
if ((Test-Path -LiteralPath $output) -or (Test-Path -LiteralPath $buildDir) -or
    (Test-Path -LiteralPath $cargoHome) -or (Test-Path -LiteralPath $ortCache) -or
    (Test-Path -LiteralPath $configFile) -or (Test-Path -LiteralPath $tauriConfig)) { throw 'native-output-exists' }

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw 'native-command-failed' }
}
Invoke-Checked 'node' @('scripts/release/native-stage.mjs', 'preflight', $SourceRevision)

# Find the default toolset in the same Visual Studio installation Cargo will use.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) { throw 'native-vswhere-missing' }
$vs = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'native-vs-missing' }
$toolset = (Get-Content -Raw -LiteralPath (Join-Path $vs 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt')).Trim()
if ($toolset -cnotmatch '^14\.\d+\.\d+$') { throw 'native-msvc-version-invalid' }
$parts = $toolset.Split('.')
$redistRoot = Join-Path $vs 'VC/Redist/MSVC'
$candidates = @(Get-ChildItem -LiteralPath $redistRoot -Directory | Where-Object {
    $_.Name -cmatch '^14\.\d+\.\d+$' -and $_.Name.StartsWith("$($parts[0]).$($parts[1]).", [StringComparison]::Ordinal)
} | Sort-Object { [version]$_.Name } -Descending)
$redistPath = $null; $redistVersion = $null; $redistDirectory = $null
foreach ($candidate in $candidates) {
    $x64 = Join-Path $candidate.FullName 'x64'
    if (-not (Test-Path -LiteralPath $x64 -PathType Container)) { continue }
    $crt = @(Get-ChildItem -LiteralPath $x64 -Directory | Where-Object { $_.Name -cmatch '^Microsoft\.VC\d+\.CRT$' })
    if ($crt.Count -ne 1) { continue }
    $required = @('msvcp140.dll', 'msvcp140_1.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')
    $allVersions = @()
    foreach ($name in $required) {
        $file = Join-Path $crt[0].FullName $name
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { $allVersions = @(); break }
        $item = Get-Item -LiteralPath $file
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'native-crt-link-invalid' }
        $version = $item.VersionInfo.FileVersion
        if ($version -cnotmatch '^(14\.\d+\.\d+)\.\d+$') { throw 'native-crt-version-invalid' }
        $allVersions += $Matches[1]
    }
    if ($allVersions.Count -ne 4 -or @($allVersions | Sort-Object -Unique).Count -ne 1) { continue }
    $actual = $allVersions[0]; $actualParts = $actual.Split('.')
    if ($actualParts[0] -cne $parts[0] -or $actualParts[1] -cne $parts[1] -or [version]$actual -lt [version]$toolset) { continue }
    $redistPath = $crt[0].FullName; $redistVersion = $actual; $redistDirectory = $candidate.Name; break
}
if (-not $redistPath) { throw 'native-compatible-crt-missing' }

# The Windows x64 Cargo feature builds the locked vendored transcribe source
# with dynamic CPU backends. A borrowed external prefix is forbidden.
$env:RUSTUP_TOOLCHAIN = '1.99.1'
$env:CARGO_TARGET_DIR = $buildDir
$env:CARGO_HOME = $cargoHome
# ort-sys verifies the downloaded archive only when its cache is absent.
# Isolate its cache so a pre-existing runner image cannot bypass that check.
$env:ORT_CACHE_DIR = $ortCache
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_INCREMENTAL = '0'
$env:NOTYPE_BUILD_SOURCE_REVISION_INPUT = $SourceRevision
$env:NOTYPE_BUILD_DIRTY_INPUT = 'false'
$metadataText = & cargo metadata --locked --format-version 1 --manifest-path 'desktop/src-tauri/Cargo.toml'
if ($LASTEXITCODE -ne 0) { throw 'native-cargo-metadata-failed' }
$metadata = ($metadataText -join "`n") | ConvertFrom-Json
$cpp = @($metadata.packages | Where-Object { $_.name -ceq 'transcribe-cpp-sys' -and $_.version -ceq '0.2.4' })
if ($cpp.Count -ne 1) { throw 'native-cpp-dependency-invalid' }
$cppRoot = Split-Path -Parent $cpp[0].manifest_path
$cppInfo = Get-Content -Raw -LiteralPath (Join-Path $cppRoot '.cargo_vcs_info.json') | ConvertFrom-Json
$cppRevision = '4807edaf210d0d7e8a6f7fb2a44b65966a2797f0'
if ($cppInfo.git.sha1 -cne $cppRevision) { throw 'native-cpp-source-invalid' }

Push-Location (Join-Path $root 'desktop')
try {
    Invoke-Checked 'bun' @('install', '--frozen-lockfile', '--ignore-scripts')
    Invoke-Checked 'bun' @('run', 'build')
    # The frontend was just rebuilt; avoid a second hidden frontend invocation.
    [IO.File]::WriteAllText($tauriConfig, '{"build":{"beforeBuildCommand":"","frontendDist":"../dist"}}', [Text.UTF8Encoding]::new($false))
    Invoke-Checked 'bun' @('run', 'tauri', 'build', '--no-bundle', '--config', $tauriConfig)
} finally { Pop-Location }

$input = [ordered]@{
    exe = (Join-Path $buildDir 'release/notype.exe')
    runtime = (Join-Path $root 'desktop/src-tauri/transcribe-libs')
    redist = $redistPath
    output = $output
    source = $SourceRevision
    cargoLock = (Join-Path $root 'desktop/src-tauri/Cargo.lock')
    bunLock = (Join-Path $root 'desktop/bun.lock')
    tools = [ordered]@{
        node = '22.15.0'; bun = '1.2.22'; rust = '1.99.1'
        msvc = $toolset; redist = $redistVersion; redist_dir = $redistDirectory
        cpp_source = 'https://github.com/handy-computer/transcribe.cpp'; cpp_revision = $cppRevision
        crt_source = 'https://learn.microsoft.com/visualstudio/releases/2022/redistribution'
    }
}
[IO.File]::WriteAllText($configFile, ($input | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
Invoke-Checked 'node' @('scripts/release/native-stage.mjs', 'stage', $configFile)
