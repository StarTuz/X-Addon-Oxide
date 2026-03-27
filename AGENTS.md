# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace. The main crates live under `crates/`:

- `crates/x-adox-core`: addon discovery, scenery management, profiles, logbook, flight generation.
- `crates/x-adox-gui`: Iced desktop UI, assets, locales, and GUI-specific docs.
- `crates/x-adox-cli`: command-line entry points for local management workflows.
- `crates/x-adox-bitnet`: heuristic scoring, aircraft classification, and NLP prompt parsing.

Supporting material lives in `docs/`, `resources/`, and `assets/packaging/`. Helper scripts are in `scripts/`. Patched upstream rendering code is vendored in `patches/iced_graphics/`. Legacy Xojo sources are preserved in `legacy_xojo/` and should only be touched for archival or migration work.

## Build, Test, and Development Commands
- `cargo build --release`: builds the full workspace.
- `cargo run --release -p x-adox-gui`: launches the desktop app locally.
- `cargo run --release -p x-adox-cli -- --root /path/to/X-Plane list`: runs the CLI against an X-Plane install.
- `cargo test`: runs all workspace tests.
- `cargo test -p x-adox-core` or `cargo test -p x-adox-bitnet`: targets one crate while iterating.
- `./scripts/local_ci.sh`: mandatory before every push; builds the release GUI binary, runs tests, and verifies the binary exists.
- `./scripts/build_appimage.sh`: builds the Linux AppImage packaging flow.

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and small focused modules. Keep errors explicit with crate-local error types or `anyhow::Result` at boundaries. Prefer descriptive test names such as `regression_simheaven.rs` or `test_parse_time_and_weather`. When touching patched renderer code, keep changes minimal and clearly scoped.

## Testing Guidelines
Most coverage lives in `crates/x-adox-core/tests/` and `crates/x-adox-bitnet/tests/`. Add regression tests beside the subsystem you change. GUI work is mostly validated indirectly, so include core or parser coverage whenever possible. If you change scenery ordering logic, run `cargo test -p x-adox-bitnet --test ordering_guardrails`. Update `docs/TESTING.md` when adding, removing, or significantly reorganizing tests.

## Commit & Pull Request Guidelines
Recent history uses Conventional Commits such as `feat(nlp): ...`, `fix: ...`, and `chore(gui): ...`. Keep commits scoped and imperative. Before opening a PR, run `./scripts/local_ci.sh`. PRs should explain the user-visible change, note any X-Plane or platform assumptions, link the relevant issue, and include screenshots for GUI changes.

## Configuration & Safety Notes
Do not commit personal X-Plane paths, generated logs, or local config. Runtime data typically lives outside the repo in the user config directory, while repo fixtures live in `resources/` and `crates/x-adox-core/data/`. Treat `scenery_packs.ini` ordering rules and BitNet scoring changes as regression-sensitive.
