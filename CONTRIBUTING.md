# Contributing

[简体中文](CONTRIBUTING.zh-CN.md)

Read [AGENTS.md](AGENTS.md) and [Technical Design](docs/technical-design.md). This repository contains one Rust 2024 Windows x64 application in `crates/bongocat-app`.

## Development

Install the prerequisites in [README](README.md). Use `just dev-fast` for incremental development, `just test` for tests, `just check` for formatting, strict Clippy, workspace tests and release compilation checks, and `just dev-smoke` for a five-second startup/shutdown check.

Smoke uses Development data and does not simulate interaction. Close existing instances first: a single-instance early exit does not prove startup worked. Manually verify panel open/close, Ctrl+Alt+L start/stop, totals after restart, dragging, wheel resizing and tray controls. Report untested behavior. Do not delete state or local backups as routine cleanup.

## Contributions

Preserve unrelated work. Fetch before integration, use a focused feature branch and Conventional Commits. Do not force-push or publish tags/releases without explicit authorization. Verify remote history before first synchronization; an empty repository has no pull-request base branch.

Keep both languages of README, this guide and CHANGELOG aligned. Each feature delivery contributes at most one Unreleased entry per language; rewrite its summary when scope changes. Preserve released history: existing 2.x history describes upstream BongoCat, not SideTick releases. Record decisions in `docs/adr/` and current architecture in `docs/technical-design.md`.

Keep Win32 code in `src/windows/`, timer logic in `src/model.rs` and persistence in `src/storage.rs`. Do not reintroduce global input tracking or continuous animation loops. Pin direct dependencies and preserve Cargo.lock. Keep executables, backups, credentials and personal state out of Git.
