# DeepSeek native console verification

Date: 2026-10-26 Asia/Taipei. Source: `41cb62da8482592bde02cc0b2b52460e361e75a9`
on `ticket/23-production-helper`; source unchanged during all commands.
Both independent final source reviews reported zero unresolved findings.

[P23-native-console-rustc-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6401664051)
received [GPT-6-Astra approval](https://github.com/swchen44/notype/issues/23#issuecomment-6401687889).
The [source gate](https://github.com/swchen44/notype/issues/23#issuecomment-6401759414)
and [execution evidence](https://github.com/swchen44/notype/issues/23#issuecomment-6402033333)
record the exact boundary and inputs.

## Actual results

The real shared console algorithm and Windows adapter compiled using installed
Rust 1.99.1 and existing hash-verified x64 Sys/Link/tempfile rlibs. No Cargo feature
unification, dependency rebuild, download or cache copy occurred. The prebuilt
Sys profile differs from the portable Cargo profile; this is explicit compiler
metadata consumption, not a Cargo cache-hit claim.

The driver executed **2 tests / 0 failures / 0 ignored**. Its owned hidden-console
matrix exercised nine cases: unicode, escape, control-key, read-error, unwind,
signal, late interrupt, disable-error and restore-error. Each child reported
restored mode and observed non-echo input; synthetic key material was absent from
receipts. A separate test rejected redirected or foreign console entry. Parent
console mode was unchanged. Explicit child completion succeeded; post-run queries
found no remaining fixture, driver, Cargo, rustc, Clippy or linker process.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Owned console child | 217088 | `cdb92d76a563dd2b139a83bbc4bc45e73dd20a1a13981d9f9c3d00f82fc0beb1` |
| Test driver | 812032 | `31f629d0da816325c1be8d352915f17b71ff818138c2c2bf0faaff5368bb4f4b` |

Both artifacts and their PDBs remain under
`D:/git/notype-build/compiler-temp/p23-native-console-41cb62da`.
The fixed external guard modes were `rustc-console-child`,
`rustc-console-driver` and `run-owned-console-driver`; no caller-supplied compiler
arguments or endpoint were accepted. The driver alone received the compile-time
`CARGO_BIN_EXE_notype-probe-console-fixture` path.

## Resource evidence and limits

The same v2 baseline/ledger retained its 192 MiB cap and 160 MiB proactive stop.
Cumulative high-water allocation/volume consumption remained 37,019,648 bytes.
Active volume sampling peaked at 205 ms; independent allocation scans took
1346–1830 ms and are not described as 250 ms scans. Observed minima were
C 2,236,219,392 and D 1,338,806,272 bytes; no floor or cap was crossed. All commands
terminated and the execution lease was returned. No model, profile, App, cache
artifact or user data was removed; only owned test directories and transient
command locks were cleaned by their normal fixture/guard lifetime.

The [full Linux CI](https://github.com/swchen44/notype/actions/runs/40224287522)
also passed on exact source `41cb62d`; its evidence covers portable/core/Node
contracts. A Linux native target with zero tests is not Windows evidence.

This receipt proves the console fixture only. It does **not** prove native paths,
real CSV disk-space calls, a production helper binary, key/live readiness, Tauri
App startup, microphone, company API or other-App input. No real key was requested
or used and no paid request occurred. Cargo's native feature cascade remains
blocked; any different helper build method requires its own exact plan review.
