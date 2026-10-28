# YAML transfer Windows acceptance — 2026-10-29

Product source: `e5a8b5af3ae842fd3894ebb6ad25d634bf556304`.
Windows executable SHA256: `7284667c18c790dc26ce71a951b59a3777420fcfbf2b01018a7b6117c273da23`.
[Full CI](https://github.com/swchen44/notype/actions/runs/40675317547) succeeded.
The complete local.6 ZIP contains1733 files and independently extracted successfully;
ZIP SHA256: `32e2d3a32a9b11ff72cb63984ad4b6630bf01233a193edd2599fadd1a76c4d10`.

Computer Use used the exact packaged executable and a separate owned profile,
with no real key or speech model. Export produced a valid UTF-8 YAML8957 bytes;
the key inclusion option was unchecked and no app_keys payload was present.
A synthetic file changed only fixture-before to fixture-after. Native preview
displayed the custom-word difference, key-omission effects and explicit whole
replacement/restart notice. Cancel retained fixture-before and created no journal.
Reimport and confirmation restarted the process (PID2624 to10676), applied
fixture-after and removed the completed journal. The restarted dictionary UI
displayed fixture-after. About/Quit normally closed the fixture process.

The preview also reports the omitted versus empty app_keys field as a visible
schema difference; credential handling remains separately described by key effects.
The current test had no saved connection/key, so actual key-bearing native import
is not established by this GUI run. Its retention/binding/explicit-clear behavior
is covered by owned provider/core integration tests. Missing model/audio conflict,
busy/stale/fault/crash-prefix paths have source contracts, not every native case.

No microphone/hotkey/full ASR session, real target-App delivery, company API,
clean Windows host, macOS/Linux desktop or additional live provider generation
was tested in this acceptance. Diagnostics remained enabled without changing
privacy controls. User profile/settings/history/weights were not import targets.
