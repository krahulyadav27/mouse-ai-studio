# Mouse AI Studio

**A Windows desktop application for mouse diagnostics, performance monitoring, input testing, and AI-assisted troubleshooting.**

Mouse AI Studio is built using React, TypeScript, Tauri 2, and Rust. It provides a desktop dashboard for discovering connected mouse devices, observing pointer activity, performing manual DPI calibration, testing mouse inputs, managing profiles, and generating diagnostic reports.

The application also includes an OpenAI-powered assistant integration with secure API-key storage through Windows Credential Manager.

## Features

### 1. Mouse Device Discovery
- Discover connected mouse devices on Windows.
- Display available device information and hardware identifiers.
- Collect manufacturer and product information when exposed by Windows.
- Display available VID/PID and device interface metadata.
- Refresh device discovery information.

### 2. DPI Calibration
- Manually calibrate mouse movement.
- Calculate estimated DPI using physical movement distance and observed pointer travel.
- Display user-configured DPI values.
- Distinguish manual estimates from verified hardware readings.

**Note:** DPI calibration is an estimate. The application does not currently change the physical DPI setting of the mouse.

### 3. Polling Rate Monitoring
- Observe pointer event delivery timing.
- Display application-level event-rate measurements.
- Monitor pointer activity during testing.

**Note:** These measurements represent events delivered to the application, not the actual USB polling frequency of the mouse.

### 4. Sensor and Movement Testing
- Observe pointer movement along the X and Y axes.
- Monitor movement activity.
- Visualize application-level movement information.

### 5. Mouse Input Testing
- Button testing.
- Double-click testing.
- Scroll testing.
- Observe mouse events delivered to the application.

### 6. Profiles and Reports
- Create and manage named mouse profiles.
- Store local settings and profile information using SQLite.
- Export diagnostic information in JSON and CSV formats.

### 7. AI Assistant
- OpenAI API integration using the Responses API.
- Ask questions about mouse hardware, DPI, performance, and troubleshooting.
- Include available mouse-device context in AI requests.
- Maintain conversation history during the current assistant session.
- Store API credentials using Windows Credential Manager.
- Remove saved API credentials from the application.

**Important:** An OpenAI API key and available API credits are required to generate AI responses. ChatGPT subscriptions and API billing are separate. Live response testing is pending sufficient API credits.

## Technology Stack

| Technology | Purpose |
|---|---|
| React | User interface |
| TypeScript | Frontend development |
| Vite | Development server and build tooling |
| Tauri 2 | Windows desktop application framework |
| Rust | Native backend |
| SQLite | Local database |
| Windows Raw Input | Mouse device discovery |
| SetupAPI and HID | Hardware metadata |
| OpenAI API | AI assistant |
| Windows Credential Manager | Secure API-key storage |

## Application Architecture

```text
Mouse AI Studio
│
├── Frontend
│   ├── React
│   ├── TypeScript
│   └── Vite
│
├── Desktop Framework
│   └── Tauri 2
│
├── Native Backend
│   └── Rust
│       ├── Windows Raw Input
│       ├── SetupAPI / HID
│       ├── SQLite commands
│       ├── Device discovery
│       └── OpenAI API integration
│
└── Local Storage
    ├── SQLite database
    └── Windows Credential Manager
```

## Project Structure

```text
mouse-ai-studio/
│
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── style.css
│   └── pages.css
│
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs
│   │   └── main.rs
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── index.html
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── .gitignore
└── README.md
```

## Windows Prerequisites

Before running the project, install:

1. Windows 10 or Windows 11.
2. Node.js.
3. Corepack / pnpm.
4. Rust stable.
5. Visual Studio C++ Build Tools.
6. Windows SDK.

Tauri's Windows development environment requires the appropriate Rust MSVC toolchain and Windows development dependencies.

## Run the Application Locally

### Step 1: Clone the repository

```powershell
git clone https://github.com/krahulyadav27/mouse-ai-studio.git
```

### Step 2: Enter the project directory

```powershell
cd mouse-ai-studio
```

### Step 3: Install dependencies

```powershell
pnpm.cmd install
```

### Step 4: Run the Windows desktop application

```powershell
pnpm.cmd tauri dev
```

This starts the Vite development server and launches Mouse AI Studio as a native Windows desktop application.

### Step 5: Check the frontend

```powershell
pnpm.cmd check
```

### Step 6: Check the Rust backend

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

## Run Frontend Only

To run the frontend without launching the Tauri desktop shell:

```powershell
pnpm.cmd dev
```

Open the local Vite address displayed in the terminal.

**Note:** Browser preview does not provide the full native Windows hardware-discovery functionality.

## Configure the AI Assistant

Mouse AI Studio supports an OpenAI-powered assistant.

1. Create an API key at https://platform.openai.com/api-keys.
2. Launch Mouse AI Studio.
3. Navigate to **AI Assistant**.
4. Enter your API key.
5. Select **Save API key securely**.
6. Start asking questions.

### API Security

- API keys are stored using Windows Credential Manager.
- API keys are not intended to be stored in frontend source code.
- Never commit API keys to GitHub.
- API usage requires separate OpenAI API billing and available credits.

## Build Windows Application

To create a production desktop build:

```powershell
pnpm.cmd tauri build
```

Tauri generates production build artifacts under:

```text
src-tauri/target/release/
```

Installer availability depends on the installed Windows bundling prerequisites and successful packaging.

**Current release status:** The application has been launched successfully in Tauri development mode. A distributable Windows installer has not yet been verified.

## Hardware Measurement Limitations

Mouse AI Studio is designed to distinguish actual hardware information from estimates and application-level observations.

- Device discovery uses Windows Raw Input, SetupAPI, and HID metadata.
- Hardware information depends on what Windows and the device driver expose.
- Polling-rate measurements represent application-level pointer event delivery.
- Movement measurements represent observed pointer travel in the application.
- Button and scroll tests observe events delivered to the WebView.
- DPI calibration is a manual estimate.
- Actual hardware DPI read/write and device-specific control protocols are not currently implemented.
- Battery readings and advanced sensor metrics are not currently available.
- The floating DPI overlay and Windows startup integration remain future improvements.

The application does not send undocumented USB commands or invent unavailable hardware readings.

## Security and Privacy

- API credentials are stored using Windows Credential Manager.
- Local profiles and settings are stored on the user's PC.
- Hardware discovery is designed to be read-only.
- No undocumented device-control commands are sent.

## Future Improvements

- Floating DPI monitoring overlay.
- Windows startup integration.
- More advanced hardware diagnostics.
- Additional mouse-device compatibility.
- Expanded performance analytics.
- Improved AI troubleshooting and recommendations.
- Production Windows installer and release packaging.

## Project Status

**Version:** 0.1.0  
**Platform:** Windows  
**Application Type:** Desktop Application

The application has passed frontend and Rust development checks and has successfully launched in Windows Tauri development mode. Some hardware features and production packaging remain under development.

## Author

**Mouse AI Studio**  
Developed as a desktop software engineering project using React, TypeScript, Rust, and Tauri.

## Repository

https://github.com/krahulyadav27/mouse-ai-studio

