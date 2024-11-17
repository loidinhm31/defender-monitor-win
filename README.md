The executable will be available at `target/release/defender-monitor.exe`

## Usage
Turn-off and check Real-time Protection for working with Intellij in WSL2
Once launched, the application will appear in your system tray with an icon indicating the current Windows Defender status:

- 🟢 Shield with checkmark: Real-time protection is enabled
- 🔴 Shield with X: Real-time protection is disabled
- ⚠️ Shield with warning: Status unknown

### System Tray Menu Options

- **Toggle Protection**: Enable or disable Windows Defender real-time protection
- **Status**: Display current protection status
- **Enable Autostart**: Configure application to start with Windows
- **Disable Autostart**: Remove application from Windows startup
- **Check Autostart Status**: Verify if application starts with Windows
- **Quit**: Exit the application

## Requirements

- Windows 10 or later
- Administrative privileges (for toggling protection status)