# notype desktop

Follow the repository-root AGENTS.md, glossary, ADRs and notype GitHub spec #8. This directory is a fork of the fixed Handy source; provenance and license are in UPSTREAM.md and LICENSE. Archived upstream guidance is historical context, not a request to work in the upstream tracker.

- New product tests belong in the repository-root tests/unittest or tests/integration directories, at the approved session/settings/UI interfaces.
- Use strict TypeScript without `any`, React function components and i18next resources for user-visible text. Preserve Rust error handling and document public interfaces.
- Run frontend lint/build, Prettier on changed frontend files and cargo fmt on changed Rust code before reporting work. Report native build/clippy/runtime checks separately when the environment prevents them.
- Desktop product builds use `tauri build --no-bundle`; installation and updates are managed through notype's npm payload flow.
