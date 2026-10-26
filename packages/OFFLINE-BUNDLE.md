# Complete offline ZIP contract (#29)

Approved [P29-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391),
[GPT-6-Astra approval](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255).
This is the source framework. Actual full payload assembly, native build,
clean-host startup, disconnected network observation and company service
acceptance remain separate P14 gates. Synthetic files establish no runtime or
model quality result.

The ZIP has a single flat delivery root:

```text
Start-notype.cmd
README-zh-TW.md
bundle.json
SHA256SUMS.txt
runtime/node.exe
runtime/LICENSE.txt
runtime/provenance.json
app/bin/notype.mjs
app/lib/*.mjs
app/package.json
app/release.json
app/node_modules/notype-win32-x64/{package.json,payload.json,...full App/runtime}
models/{package.json,payload.json,NOTICE.txt,...flat model and notices}
bootstrap/notype-bootstrap.json                 # optional
```

The facade has only its pinned application dependency, no model dependency,
optional dependency or lifecycle scripts. `release.json` retains schema2 and
omits its npm `model` pin in ZIP mode: the separate bundle pin owns `models/`.
The model payload uses the existing package schema1 with flat activation source
and the same catalog model ID as the bundle. There is no activation copy,
hardlink, repair, installer, download, package-manager call or vendor script at
startup. The program does not move an existing model folder.

`bundle.json` is bounded to 4 MiB. It accepts exactly these fields:

| Field                     | Contract                                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `schema`, `mode`          | `1`, `"offline-bundle"`                                                                                                         |
| `id`, `version`, `source` | Bounded public bundle ID, exact release version, final reviewed 40-hex source commit                                            |
| `facade`                  | `"app"`                                                                                                                         |
| `application`             | Existing exact `{name:"notype-win32-x64",version,manifest:"payload.json",sha256}` pin                                           |
| `model`                   | `{model_id,root:"models",pin:{name,version,manifest:"payload.json",sha256}}`                                                    |
| `node`                    | `{path:"runtime/node.exe",version:"22.15.0",sha256}`                                                                            |
| `bootstrap`               | Optional `{path:"bootstrap/notype-bootstrap.json",sha256}`                                                                      |
| `files`                   | Exact payload inventory of `{path,bytes,sha256,role,source}`; positive sizes, lowercase SHA256, no duplicate Windows-case paths |
| `sources`                 | Named `{source,revision,license,notices}` provenance records; public credential-free HTTPS URLs and declared hashed notices     |

File roles are `launcher`, `resource`, `metadata`, `node`, `native`, `runtime`,
`model`, `model-support`, `notice`, `provenance`, `bootstrap`, `entrypoint`,
`host-probe`, and `webview-entry`. Every file references a declared source.
The Node executable uses role `node`, the fixed Start file `launcher`, runtime
provenance `provenance`, optional bootstrap `bootstrap`, and license text
`notice`. Required Node notice and provenance cannot be omitted. Full App
verification reuses the pinned package verifier; the current offline release
also requires Fixed WebView `156.0.4314.42` and exactly 257 independently
inventoried files under its declared directory. Count alone is insufficient:
all declared App/runtime bytes and notices are hashed. Changing these fixed
runtime/Node pins requires reviewed inputs and notices.

`bundle.json` and `SHA256SUMS.txt` are the **only two reserved metadata files**.
They are not recursively included in `files`. The checksum file must equal
canonical UTF-8/LF lines `sha256 + two spaces + relative path + LF`, sorted by
case-sensitive code-point path order. It lists every payload file plus the
actual hash of `bundle.json`. It does not list itself. Both reserved files must
exist, must be regular files, and are included in exact archive inventory and
external archive hash. Archive acceptance therefore has no recursive self-hash
claim or arbitrary metadata ignore rule. Hashes detect corruption, not a party
rewriting the entire bundle and its manifests.

`startCommand()` produces the exact CRLF launcher. It disables delayed
expansion, quotes adjacent `%~dp0runtime\node.exe` and
`%~dp0app\bin\notype.mjs`, and passes only `--offline-bundle`. It never forwards
`%*`. The facade derives its root from `import.meta.url` through its fixed `app`
location, independently of cwd/PATH or environment. It compares canonical
`process.execPath` to the verified adjacent runtime. No quoted directory ending
in a backslash is passed as an argv value. All native invocation uses absolute
paths and `shell:false`.

`inspectBundle({facadeRoot,execPath,strictModels})` is read-only. Runtime launch
always uses the actual process executable; `execPath` is a public system adapter
seam for owned runtime doubles. `inspectRelease`/`--offline-bundle --check`
strictly verify every declared model, notice, bootstrap, App and runtime file.
Normal `prepareLaunch` hard-verifies all program/runtime/Node/bootstrap inputs,
CPU, token and separate persistent data path. An absent or invalid optional
`models/` package, including its notices, makes the bundled model unavailable;
it never declares full bundle integrity or substitutes for an explicit saved
user root. Links, reparse ancestry, hardlinks, unlisted/private files and unsafe
paths remain hard failures. Native verifies the effective model using its
RootService before loading it.

The only native handoff is `--offline-bundle-root <canonical absolute root>`.
Inherited `NOTYPE_BOOTSTRAP*`, `NOTYPE_BUNDLE*`, `NOTYPE_OFFLINE*` and
`NOTYPE_MODEL*` are removed, alongside the existing fixed-runtime overrides.
Native independently checks bundle ownership, pins and bootstrap and performs
all domain initialization; the facade has no provider/selection store or key
reader. See the [shared bootstrap contract](../docs/offline-bootstrap.md).
The optional bootstrap transport is limited to 64 KiB, rejects unknown/secret
fields, credential URLs and environment expansion, and cannot include backend
IDs or key actions. Native draft/capability validation is authoritative.
[bootstrap.example.json](notype/bootstrap.example.json) is synthetic
administrator input, not an invented company configuration and not an automatic
generic ZIP preset. A generic ZIP may omit bootstrap and use its declared model
through normal first-run setup. End users enter their keys through the existing
write-only App control.

`assemblyInventory(root)` counts actual regular file bytes before packing and
allows audited immutable staging hardlinks. `bundleInventory(root)` and launch
verification always reject links and `nlink>1`; no CLI flag weakens this check.
The packer must write ordinary ZIP file bytes, retain the whole approved vendor
runtime and preserve source bytes. These functions do not acquire or assemble
large inputs. P14 still owns fresh-space admission and exact source provenance.

`inspectZipArchive(zip,{expectedFiles})` accepts a mandatory trusted exact
inventory including the two reserved files, each with path, size and hash.
It validates ZIP32 central and local records **before extraction**: relative
Windows-safe UTF-8 paths, consistent directory casing, exact inventory, no
duplicate/case alias, links, nonregular file modes, extra fields, encrypted
entries, unknown method, overlapping data, prepended/appended payload or
unbounded declared expansion. Stored/deflate entries are streamed with an
expanded-byte limit, SHA256 and CRC validation; the whole archive SHA256 is
streamed separately. Supported ZIPs have no comments, data descriptors or ZIP64
extensions. The current complete bundle fits ZIP32; a larger delivery requires
a reviewed format extension. No extraction, temp archive copy, network or
source write occurs in this seam. After audit, the delivery process still
extracts into a fresh owned root and runs full extracted inventory verification.

Source tests use real tiny ZIPs created by installed Python standard-library
`zipfile`, then owned extraction, production verification and an owned sentinel
child. Windows additionally invokes the actual `.cmd` with a compiled tiny
runtime double under an empty PATH, Unicode/spaces/ampersand and wrong cwd.
This is distinct from actual Node/native/clean-host acceptance. Source fixtures
contain synthetic 257-file runtime bytes, no real App/model/Node payload,
company values, credentials, data/profile, microphone or private endpoint.
