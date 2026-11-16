# SideTick

[简体中文](README.zh-CN.md)

A lightweight Windows desktop companion with a static cat avatar and a cumulative work timer. Based on BongoCat; executable and some system labels still use the BongoCat name.

## Use

| Action | Result |
| --- | --- |
| Right-click avatar | Toggle translucent time panel |
| Panel button or Ctrl+Alt+L | Start/stop work session |
| Left-drag | Move avatar |
| Scroll over avatar | Resize, from 25% to 400% |
| Right-drag | Resize without opening panel |
| Right-click tray icon | Timer, always-on-top, size, opacity and exit controls |

**Days** starts at 0 and increases with elapsed local calendar dates. It is not measured internet usage. **Hours** shows cumulative work time rounded to one decimal place. Paused time is excluded; restarting restores the total with the timer paused. Normal exit saves the session; unexpected termination may lose time since the latest periodic save (normally every five seconds).

Right-click again, press Esc while the panel has focus, or click another application to dismiss it. Closing the panel does not stop an active timer. The avatar does not animate or respond to general keyboard/mouse activity. Ctrl+Alt+L is fixed and must be available for startup. The current UI is Chinese.

## Build and run

Use Windows x64, the Rust MSVC toolchain in `rust-toolchain.toml`, Microsoft C++ Build Tools with Windows SDK, and `just` on PATH. Windows 10 1903+ is the intended minimum; other operating systems are not supported.

```powershell
just check
just dev-smoke
just dev-fast
```

`dev-fast` builds and starts Debug Development, reusing incremental caches. Without code changes, run `target\debug\bongocat-app.exe` directly. Exit through the tray before rebuilding.

`just build` creates the Production executable `target\release\bongocat-app.exe`, not an installer or release. The release check inside `just check` does not replace this executable.

## Data

State lives under the Windows roaming application-data folder:

- Development: `com.ayangweb.bongo-cat\development\personal-state.json`
- Production: `com.ayangweb.bongo-cat\production\personal-state.json`

Totals, settings and single-instance locks are isolated by environment. No older BongoCat configuration or work-time data is imported. Preserve the state file to retain totals and back it up before manual changes. Invalid state causes a startup error and is not automatically overwritten.

## Project

[Repository](https://github.com/anleatr/SideTick) · [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

Derived from [BongoCat](https://github.com/ayangweb/BongoCat); the static avatar comes from its existing model cover resource. Original license and attribution requirements still apply; see [LICENSE](LICENSE). Cubism Core and Live2D models are not bundled.
