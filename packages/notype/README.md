# notype launcher foundation

Private packaging source, not an assembled application release. `notype` refuses to launch until a complete verified release is assembled. `notype --check` inspects installed package integrity without starting the app; it does not certify host compatibility or model quality.

Only Windows x64 is supported by the first payload. Launch requires a normal Windows user token; elevated launches fail clearly. No privilege switching, install scripts, native builds, artifact downloads or Evergreen fallback are provided.

The default persistent root matches the existing app: `%APPDATA%/io.github.swchen44.notype`. An absolute local `NOTYPE_DATA_DIR` override is preserved. History, settings, models and WebView user data stay outside versioned npm packages. Updating/removing the program package must not delete those files.

Application/runtime/model package names are provisional private roles. Actual pins, full source/third-party notices, CPU compatibility, local model activation and native launch remain release integration work. The repository's root private Node package is a separate diagnostic prototype.
