# Mezon Tauri Desktop App

A lightweight, fast desktop application for Mezon built with [Tauri](https://tauri.app/).

## Prerequisites

- **Node.js** >= 18.x
- **Yarn** >= 1.22.4
- **Rust** >= 1.70.0
- **Tauri CLI** (installed via cargo)

### System Dependencies

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

#### Fedora
```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel
sudo dnf group install "C Development Tools and Libraries"
```

#### macOS
```bash
xcode-select --install
```

#### Windows
- Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## Installation

```bash
# Install Tauri CLI
cargo install tauri-cli

# Install project dependencies
yarn install
```

## Development

```bash
# Start development mode (hot reload)
yarn dev:tauri

# Or using Nx
nx dev tauri
```

## Build

```bash
# Build for production
yarn build:tauri

# Build debug version
yarn build:tauri:debug

# Or using Nx
nx build tauri
```

## Output

Built applications will be in:
- `apps/tauri/src-tauri/target/release/bundle/`

Supported formats:
- **Windows**: NSIS installer (.exe), MSI installer
- **macOS**: DMG, App bundle
- **Linux**: DEB, RPM, AppImage

## Architecture

```
apps/tauri/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Main Tauri application
│   │   └── commands.rs      # Custom Tauri commands
│   ├── icons/               # Application icons
│   ├── capabilities/        # Tauri v2 capabilities
│   ├── Cargo.toml           # Rust dependencies
│   ├── build.rs             # Build script
│   └── tauri.conf.json      # Tauri configuration
├── project.json             # Nx project configuration
└── README.md
```

## Comparison with Electron

| Feature | Tauri | Electron |
|---------|-------|----------|
| Bundle Size | ~3-10 MB | ~150+ MB |
| Memory Usage | Lower | Higher |
| Startup Time | Faster | Slower |
| Backend | Rust | Node.js |
| WebView | System WebView | Chromium |

## Features

- **System Tray**: Minimize to tray, tray menu
- **Deep Linking**: `mezonapp://` protocol handler
- **Auto Updates**: Built-in update mechanism
- **Single Instance**: Prevents multiple app instances
- **Window State**: Remembers window size/position
- **Notifications**: Native system notifications
- **Clipboard**: Read/write clipboard content
- **Global Shortcuts**: Register global keyboard shortcuts

## License

MIT
