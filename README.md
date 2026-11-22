# Mouse AI Studio

Mouse AI Studio is the existing React + TypeScript dashboard, prepared to run inside a Tauri 2 Windows desktop shell. Hardware fields are only shown when Windows reports them; unavailable fields stay unavailable. The Windows discovery implementation uses Raw Input to enumerate present mouse collections and SetupAPI/HID to collect matching interface metadata. It opens HID collections read-only for descriptor strings and does not send output or feature reports.

## Implemented in the project

- Existing React/Vite interface is retained, including dashboard navigation, mouse discovery view, DPI calibration, polling observation, movement observation, button, double-click and scroll tests, profiles, reports and settings.
- Tauri 2 application configuration with Mouse AI Studio name, version `0.1.0`, Windows window dimensions, application icons, NSIS/MSI bundle targets and tray menu source.
- Rust Windows device enumeration source using Raw Input, SetupAPI and HID descriptors, including manufacturer/product strings when exposed, VID/PID and hardware IDs when present, connection hints, device paths and a two-second reconnection scan.
- Tray menu source for showing the window and quitting. Closing the main window hides it to the tray.
- SQLite schema and Tauri commands for local settings and named DPI-value profiles.
- Browser-facing input tests use actual events delivered to the WebView and explain their measurement limits. The browser preview does not claim to detect hardware.
- A generated Mouse AI Studio icon is included in `src-tauri/icons/` and referenced by the Tauri bundle configuration.

## Not implemented or not verified

- **No Windows desktop binary or installer was produced.** The Tauri build reached Rust release compilation, but Windows Application Control denied execution of generated Rust build-script executables with OS error `4551`. This prevents verifying or launching the native app in this environment. NSIS was also not found, so an NSIS setup executable cannot be packaged here. No `.exe` or `.msi` installer exists yet.
- The Rust backend, SQLite commands, tray behavior, live device discovery and installer still need a successful native Windows build and runtime verification. Frontend build success does not verify Rust compilation.
- Start with Windows and the floating DPI overlay remain pending. Window close-to-tray and tray menu behavior are implemented in native source but are not runtime verified.
- Raw Input device arrival/removal is observed by periodic re-enumeration; a native `WM_INPUT_DEVICE_CHANGE` event handler is not implemented. Tests currently observe WebView events; raw HID input reports are not captured.
- Actual mouse DPI read/write, physical DPI-button event identification, hardware stages and device-specific profile application are not implemented. The Arctic Fox Clearview product page lists 1600 DPI resolution, but the mouse has not been identified among the currently connected devices and no supported DPI control protocol has been verified. The application intentionally does not write undocumented USB commands. The configured DPI is only a local profile value; manual calibration is an estimate.
- AI service integration, battery readings and hardware sensor metrics remain pending.

## Windows prerequisites

- Windows 10 or 11.
- Node.js and Corepack/pnpm. This environment has Node `v24.22.1` and pnpm `11.25.0`; `npm` is not on PATH.
- Rust stable with the MSVC target, Visual Studio C++ Build Tools and Windows SDK.
- To package setup installers, install NSIS for `-b nsis` and the WiX Toolset for `-b msi` as required by Tauri's Windows bundler.
- The machine's application-control policy must allow Cargo/Rust build scripts and proc-macro executables to run.

## Run locally

From this directory in PowerShell:

```powershell
corepack pnpm install
corepack pnpm tauri:dev
```

The Vite/Tauri development URL is `http://127.0.0.1:1420/`. Tauri starts Vite with strict port binding, so it exits clearly if that port is already occupied. For frontend-only preview:

```powershell
corepack pnpm dev
```

In this managed environment, the pnpm launcher has failed while creating a symlink in its AppData package store (`EPERM`). The installed dependencies were usable with these direct Node commands:

```powershell
node node_modules/typescript/bin/tsc -b
node node_modules/vite/bin/vite.js build
node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 1420 --strictPort
```

The TypeScript check and Vite production build pass. The UI has not been verified in a live browser session here.

## Build Windows installers

After installing the prerequisites above and allowing Cargo build subprocesses, run:

```powershell
corepack pnpm tauri:build -- --bundles nsis,msi
```

Tauri writes bundles under `src-tauri/target/release/bundle/`. Expected files include a setup `.exe` in `nsis/` and an `.msi` in `msi/`, depending on installed bundler prerequisites. No installer file is currently available to provide.

## Hardware measurement notes

- Raw Input identifies mouse input collections and device paths. Windows/driver metadata may not expose a retail model name, current DPI, sensor, battery or firmware.
- Polling-rate view measures pointer event delivery timing inside the WebView, not the USB/HID report rate.
- Movement view accumulates observed CSS-pixel travel; it cannot measure sensor jitter, lift-off distance or surface accuracy.
- Button, double-click and scroll views measure events delivered to the app, not switch debounce or low-level hardware timing.
- DPI calibration uses the user's entered physical distance and pointer travel. It is an estimate and does not change hardware DPI or Windows pointer speed.
- No sample readings are generated automatically.
