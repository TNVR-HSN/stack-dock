# Stack Dock

**A customizable Windows dock for organizing and launching shortcuts in expandable grids.**

[![Platform — Windows](https://img.shields.io/badge/platform-Windows-0078D4?logo=windows)](#requirements)
[![Built with Tauri 2](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?logo=tauri)](#project-structure)
[![Status — Pre-release](https://img.shields.io/badge/status-pre--release-F2A900)](#project-status)

[Features](#features) · [Install](#install-a-release) · [Build from source](#run-from-source) · [Usage](#usage) · [Contributing](#contributing)

Stack Dock keeps your desktop tidy without putting all your shortcuts in one long bar. Add a folder to the dock, click it to open its **Grid Stack**, and arrange apps, games, files, folders, and links in the cells you want. The dock stays compact when the grid is closed.

<!--
Before publishing screenshots, add an image at docs/images/stack-dock.png and uncomment this Markdown line.
![Stack Dock on Windows with an expanded Grid Stack](docs/images/stack-dock.png)
A short clip showing closed dock → open grid → launch indicator also works well. Remove personal paths, desktop files, and private URLs from captures.
-->

## Project status

Stack Dock is in pre-release development. **Windows is the supported platform for the desktop build.** macOS and Linux are not supported targets for the Windows-specific dock behavior. Check the repository’s **Releases** tab for published installers. If no installer is listed, build from source.

The **Hover & Settings** update adds instant hover titles, delayed target details, configurable hover motion, movable Settings and Shortcut Properties windows, and a reorganized Settings interface with a General tab. It also adds column editing and adjustable shadow placement. Panel transparency is controlled by opacity. Backdrop blur is not used.

## Features

| Area | What you can do |
| --- | --- |
| **Dock and Grid Stacks** | Place the dock on any screen edge and choose its alignment. Give each folder its own grid, icon, layout, and appearance. |
| **Shortcut targets** | Run an EXE with optional arguments and a working directory. Open local files and folders, web links, or application links such as `steam://` when the handler is installed and allowed by the app configuration. |
| **Grid editing** | Drag shortcuts between cells, choose a destination from the context menu, insert rows and columns, and remove empty ones. Auto-fill lets shortcuts wrap across columns. Row-wise layout keeps rows grouped. |
| **Custom icons** | Pick a PNG, JPEG, GIF, WebP, BMP, ICO, or an embedded EXE icon. Choose from available icon groups in an EXE or reset to the target icon or default. |
| **Hover and launch feedback** | See the full item name immediately and the shortcut target after a configurable pause. Customize hover scale, lift, tilt, timing, highlight, reflections, and ground shadows. The optional launch indicator shows which shortcut was opened. |
| **Appearance and positioning** | Set color, opacity, borders, icon roundness, labels, spacing, and padding. Split horizontal and vertical spacing or padding when needed. Settings includes a live preview. |
| **Everyday tools** | Search across folders, export or import a JSON layout, and optionally start Stack Dock at Windows sign-in from an installed release build. |

## Requirements

- **To run an installer** — a Windows PC with the Microsoft Edge WebView2 Runtime. You do not need Node.js or Rust to run an installed release.
- **To build from source** — Git (or a source ZIP), Node.js with npm, a current stable Rust **MSVC** toolchain, Microsoft C++ Build Tools with **Desktop development with C++**, and WebView2. Use versions compatible with the project's dependency lockfiles.

See the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) for setup details.

## Get started

### Install a release

If an installer has been published, open **Releases**, choose the installer for your Windows architecture, and launch **Stack Dock** from the Start menu after setup. A GitHub **Source code** ZIP is not an installer.

### Run from source

Clone this repository using **Code** on GitHub or extract the full source ZIP. From the project root, where `package.json` and `src-tauri/` are located, run:

```powershell
npm ci
npm run tauri dev
```

Use `npm install` instead of `npm ci` if your source checkout does not have a `package-lock.json`. The Tauri dev command starts Vite and opens the Windows desktop app. Browser-only Vite previews do not have native launching, Windows file pickers, or startup integration.

### Build a Windows installer

Install the dependencies as above, then run:

```powershell
npm run tauri build
```

With the default target directory, look in `src-tauri/target/release/bundle/` for the installer output. The files generated depend on the configured bundle targets. See the [Tauri Windows installer guide](https://v2.tauri.app/distribute/windows-installer/) for NSIS and MSI details.

## Usage

### Your first Grid Stack

1. Click **Shortcuts**, the empty folder created on first run.
2. Right-click an empty cell and choose **Add shortcut here…**. You can also drop supported files from Explorer onto the grid.
3. Give the shortcut a display name, choose its target type, and save. Click the new shortcut to launch it.
4. Right-click the collapsed dock and select **New folder…** to create another stack. For positioning, use **Screen Position** in the same menu or **Settings → Dock → Layout**.

| Target type | Example | Notes |
| --- | --- | --- |
| **Program** | `C:GamesMyGameGame.exe` | Use a full EXE path. Put launch flags in **Arguments** and an existing working folder in **Start in**, if needed. |
| **File or folder** | `D:GamesScreenshots` | Opens the existing path using Windows. |
| **Web link** | `https://example.com` or `steam://rungameid/228300` | Include the scheme. Custom links require a registered handler and an applicable opener permission. |

To edit an item, right-click it and choose **Shortcut properties…**. Use the browse button next to **Icon path**, choose an embedded icon if offered, or use reset to go back to the target EXE icon or the default. Imported icons are stored as static PNGs no larger than 256 px on the longest side. Stack Dock reads an EXE's icon without running the EXE.

### Settings and layout

| Settings tab | Controls |
| --- | --- |
| **General** | Windows sign-in startup, hover names and target delay, grid focus behavior, transition speed, and launch indicator. The default target delay is 1.5 seconds. |
| **Dock** | Screen edge and alignment, icon size, spacing and padding, panel appearance, and hover effects. |
| **Folder** | Folder name, icon, and optional folder-specific dock icon appearance. |
| **Grid Stack** | Layout mode, dimensions, labels, panel appearance, and effects for the selected folder. |

Drag the title area to move Settings or Shortcut Properties. In a grid, right-click to **Insert row below**, **Insert column right**, or remove an empty row or column. A row or column containing a shortcut cannot be removed with the empty-row or empty-column action.

### Keyboard and menu shortcuts

| Action | Control |
| --- | --- |
| Search by shortcut name, folder, or target | **Ctrl+F** while Stack Dock has focus, or **Find shortcut…** in the dock menu |
| Navigate search results | **↑** / **↓** to select, **Enter** to launch |
| Open the selected item's context menu | **Shift+F10** or the **Menu** key when a dock or grid control has focus |
| Dismiss a menu or close the grid | **Esc** |
| Quit Stack Dock | Right-click the collapsed dock → **Stack Dock → Exit** |

### Startup and backups

- **Start at sign-in** — With an installed release, open **Settings → General → Behavior**, enable **Start Stack Dock when I sign in**, and save. The dev build does not register itself to start because it depends on the dev server.
- **Export** — Right-click the collapsed dock and choose **Stack Dock → Export layout…** to save the layout as JSON.
- **Import** — Choose **Stack Dock → Import layout…** and confirm replacement. Export your current layout first if you want to keep it. Backups store paths and URLs as entered. If you move to another PC or drive, edit shortcuts whose targets have moved. Importing a layout does not enable or disable Windows startup.

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| The frontend opens in a browser, but clicking a shortcut does nothing | Run `npm run tauri dev` and use the desktop window. Native operations do not run in the browser preview. |
| Tauri cannot find the linker or fails to compile Rust dependencies | Check the Visual Studio C++ workload and MSVC Rust toolchain, then run `npm run tauri info` from the project root. |
| A program or local file will not open | Check the full target path and whether the file still exists. Keep program flags in **Arguments**, not in the target path. |
| A `steam://` or other app link will not open | Check that the handler is installed, the link works in Windows, and the Tauri opener permissions allow that scheme. |
| The startup switch cannot be enabled | Use an installed release instead of the development build. |
| An icon will not load | Confirm the file is a supported image or an EXE with an embedded icon. Try a PNG or ICO if the EXE has no readable icon. |
| The dock looks clipped, displaced, or hidden | Include your screen resolution, DPI scaling, monitor arrangement, dock edge, and taskbar position or auto-hide setting when reporting it. |
| An imported layout has broken shortcuts | The export contains references, not the target files. Replace paths that differ on the destination PC. |

## Project structure

Stack Dock uses **Tauri 2**, **Rust**, **JavaScript**, **HTML/CSS**, and **Vite**. Windows integration sits in the Tauri backend.

| Path | Role |
| --- | --- |
| `index.html` | Main UI, grid behavior, settings forms, and layout state |
| `public/stack-dock-native.js` | Connects the UI to native pickers, Properties, shortcut search, and layout backups |
| `public/stack-dock-widget.js` | Hover and launch effects, grid interactions, and Windows work-area placement |
| `src-tauri/src/lib.rs` | Registers Tauri commands and handles native launching, window clipping, and startup |
| `src-tauri/src/native_shortcuts.rs` | Explorer dialogs, icon decoding, drag-and-drop inspection, and backup file I/O |
| `src-tauri/src/desktop_widget.rs` | Windows-specific taskbar and dock window integration |
| `src-tauri/tauri.conf.json` and `src-tauri/capabilities/` | Desktop window, bundling, and permission configuration |

## Data and privacy

Stack Dock saves layouts locally in the desktop app's WebView storage. A layout can contain full file paths, working directories, launch arguments, URLs, and embedded custom icons. No account or cloud sync is required. Opening a shortcut can start a local executable or hand a URL to another app, so review layouts you receive before launching their entries. Redact paths, arguments, and URLs before sharing a backup, screenshot, or log publicly.

## Support and security

Use **Issues** for reproducible bugs and feature requests. If you believe you found a security issue, avoid posting executable payloads or private paths publicly. Use GitHub’s private vulnerability reporting if it is enabled for this repository.

## Contributing

Bug reports, feature suggestions, and focused pull requests are welcome. Search existing **Issues** first so related reports stay together.

- **Report a bug** with your Windows version, Stack Dock version or commit, and whether it happened in a dev or installed build. Include steps to reproduce, expected and actual behavior, and a screenshot or relevant log if useful.
- **Suggest a feature** by describing what you are trying to do and where the current workflow gets in the way. A mockup or concrete example helps with changes to dock and grid behavior.
- **Submit a pull request** from a focused branch. Explain what changed and how you tested it. For UI changes, include before-and-after captures if they help explain the change.

For changes that touch native behavior, test with `npm run tauri dev` on Windows. Useful regression checks include opening and moving dialogs, launching different target types, drag and drop, backing up and restoring a layout, and positioning the dock at different screen edges and DPI settings. Test sign-in startup with an installed release. Avoid committing `node_modules/`, `dist/`, `src-tauri/target/`, credentials, or personal layout exports.

## Release notes and roadmap

Published builds should list changes and known issues in GitHub **Releases**. Near-term work is focused on Windows testing across different monitor and taskbar setups, a clean install and upgrade path, and feedback from early users. Ideas and requests belong in **Issues**.

## Acknowledgments

Stack Dock is built with [Tauri](https://tauri.app/), [Rust](https://www.rust-lang.org/), [Vite](https://vite.dev/), and [Microsoft Edge WebView2](https://developer.microsoft.com/microsoft-edge/webview2/). It uses the Rust [image](https://crates.io/crates/image) and [icoextract_rs](https://crates.io/crates/icoextract_rs) crates for icon handling. Third-party product names identify compatible software and link handlers and do not imply endorsement.

## License

See the repository’s `LICENSE` file if one is provided. A public repository is not itself a license to redistribute or reuse the code. Third-party dependencies are governed by their own licenses.

<!--
Maintainer checklist before public release
- Add a repository description and topics such as windows, tauri, rust, dock, and launcher.
- Add an approved screenshot, an actual LICENSE once chosen, and a link to it above.
- Publish a tagged GitHub release only after testing the installer. Then add a release badge, a direct Releases link, and tested Windows versions/architectures.
- If you want separate community files, add CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, and issue/PR templates under .github, then link to the files that exist.
- Add CI, download, coverage, or license badges only when their corresponding workflow, release, report, or license is in place.
-->

[Back to top](#stack-dock)
