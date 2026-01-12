# Mezon Desktop (Tauri)

A lightweight, fast desktop application for Mezon built with [Tauri](https://tauri.app/). This is an alternative to the Electron-based desktop app with smaller bundle size and lower memory usage.

## Prerequisites

- **Node.js** >= 18.x
- **Yarn** >= 1.22.4
- **Rust** >= 1.70.0
- **Tauri CLI** (installed via cargo or yarn)

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
# Install project dependencies
yarn install

# Tauri CLI is included as a dev dependency
```

## Development

```bash
# Start development mode (hot reload)
yarn dev:desktop-tauri

# Or using Nx
nx dev desktop-tauri
```

## Build

```bash
# Build for production
yarn build:desktop-tauri

# Build debug version
yarn build:desktop-tauri:debug

# Or using Nx
nx build desktop-tauri
```

## Output

Built applications will be in:
- `apps/desktop-tauri/src-tauri/target/release/bundle/`

Supported formats:
- **Windows**: NSIS installer (.exe), MSI installer
- **macOS**: DMG, App bundle
- **Linux**: DEB, RPM, AppImage

## Architecture

```
apps/desktop-tauri/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Main Tauri application
│   │   ├── commands.rs      # Custom Tauri commands (IPC handlers)
│   │   └── window.rs        # Window management utilities
│   ├── assets/              # Desktop assets (copied from Electron app)
│   ├── icons/               # Application icons
│   ├── capabilities/        # Tauri v2 security permissions
│   ├── Cargo.toml           # Rust dependencies
│   ├── build.rs             # Build script
│   └── tauri.conf.json      # Tauri configuration
├── project.json             # Nx project configuration
└── README.md
```

## Comparison with Electron Desktop App

| Feature | Tauri (desktop-tauri) | Electron (desktop) |
|---------|----------------------|-------------------|
| Bundle Size | ~5-10 MB | ~150+ MB |
| Memory Usage | Lower | Higher |
| Startup Time | Faster | Slower |
| Backend | Rust | Node.js |
| WebView | System WebView | Chromium |
| Cross-Platform | Yes | Yes |

## Features (Matching Electron Desktop)

### Window Management
- Frameless window with custom title bar
- Minimize/Maximize/Close actions
- Window state persistence
- Dynamic window title

### System Integration
- **System Tray**: Show/Hide/Quit menu, click to restore
- **Deep Linking**: `mezonapp://` protocol handler
- **Single Instance**: Prevents multiple app instances
- **Auto Startup**: Launch on system boot

### Notifications & Badge
- Native system notifications with click handlers
- Dock/Taskbar badge count (macOS, Windows overlay)

### File Operations
- File download with save dialog
- Clipboard read/write (text and images)

### Updates
- Auto-update checking and installation
- Update progress notifications

### Media Permissions (macOS)
- Microphone permission handling
- Camera permission handling

### Global Shortcuts
- Cmd/Ctrl+, for Settings

### Events (Frontend Integration)
- `window-focused` / `window-blurred`
- `deep-link` for URL handling
- `check-updates` trigger
- `trigger-shortcut` for keyboard shortcuts
- `notification-data` for notification click handling
- `update-progress` for download progress
- `reload-app` trigger

## Tauri Commands (IPC API)

```typescript
// App Info
invoke('get_app_version')
invoke('get_platform')
invoke('get_device_id')
invoke('get_sender_id')

// Window Management
invoke('title_bar_action', { action: 'minimize' | 'maximize' | 'close' | 'hide' })
invoke('get_window_state')
invoke('set_window_title', { title: 'New Title' })
invoke('set_ratio_window', { ratio: 1.0 })
invoke('minimize_to_tray')
invoke('quit_app')
invoke('reload_app')

// Badge
invoke('set_badge_count', { count: 5 })
invoke('clear_badge')

// Notifications
invoke('show_notification', { options: { title, body, silent?, data? } })

// Files
invoke('download_file', { options: { url, filename? } })
invoke('save_file_dialog', { default_name?, filters? })

// Clipboard
invoke('copy_to_clipboard', { text: 'content' })
invoke('copy_image_to_clipboard', { url: 'image_url' })
invoke('read_clipboard')

// Image Viewer
invoke('open_image_window', { options: { url, filename?, attachments? } })
invoke('handle_action_show_image', { action: 'copyLink' | 'copyImage' | 'openLink' | 'saveImage', url })

// Updates
invoke('check_update')
invoke('install_update')

// Permissions (macOS)
invoke('request_permission_microphone')
invoke('request_permission_camera')
invoke('check_permission_microphone')
invoke('check_permission_camera')

// Activity Tracking
invoke('update_activity_tracking', { enabled: true })
invoke('get_active_window')
```

## License

MIT
