# StackDock Documentation

StackDock is a Windows desktop dock for opening apps, games, files, folders, and links. Put your most-used shortcuts directly on the dock. Group the rest into folders that open into **Grid Stacks**, where each shortcut stays in the cell you choose.

**New to StackDock?** Follow [Install or run StackDock](#install-or-run-stackdock) and [Quick start](#quick-start). If you already use it, see [Back up and restore](#back-up-and-restore) before upgrading or resetting.

[Downloads and release notes](https://github.com/TNVR-HSN/stack-dock/releases) · [Report an issue](https://github.com/TNVR-HSN/stack-dock/issues)

## Contents

- [Install or run StackDock](#install-or-run-stackdock)
- [Quick start](#quick-start)
- [Shortcuts and icons](#shortcuts-and-icons)
- [Organize the dock and Grid Stacks](#organize-the-dock-and-grid-stacks)
- [Customize StackDock](#customize-stackdock)
- [Search, keyboard controls, and startup](#search-keyboard-controls-and-startup)
- [Back up and restore](#back-up-and-restore)
- [Troubleshooting](#troubleshooting)
- [Developer and release guide](#developer-and-release-guide)
- [Support and license](#support-and-license)

## Install or run StackDock

StackDock is a **Windows desktop app**. Choose a download compatible with your Windows architecture. It uses the **Microsoft Edge WebView2 Runtime**. You don’t need Node.js, Rust, or Visual Studio to run a release build. The Windows dock features aren’t supported on macOS or Linux. [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)

Use the files attached to the release under **Assets**. Not every release necessarily includes every format.

| Download | What to do |
| --- | --- |
| **Setup EXE** (`*-setup.exe`) | Run the installer and follow the prompts. This is the simplest choice for a regular installation. |
| **MSI** (`*.msi`) | Open the Windows Installer package and follow the prompts. Use this if you prefer an MSI package. |
| **Portable ZIP** | Extract the whole ZIP to a folder you can write to, then run `StackDock.exe`. There’s no installation wizard. Keep the EXE where you intend to use it. |
| **Source code ZIP** | This is the project source, not a ready-to-run app. See [Run from source](#run-from-source). |

The portable download is **installer-free**. Its layout still lives in local WebView storage for your Windows user, not next to the EXE. To move your layout to a different PC, export and import it. Don’t assume the installed and standalone copies give you separate layouts.

**Updating or removing StackDock.** Export a layout backup first, then use **StackDock → Exit** in the collapsed dock’s right-click menu. Install the newer release or replace the EXE in the extracted portable folder. To uninstall an installed copy, use **Settings → Apps → Installed apps** in Windows. To remove a portable copy, delete its extracted files after exiting. If you enabled sign-in startup, turn it off in StackDock before moving or removing its EXE.

## Quick start

1. Launch StackDock. On a fresh layout, **Shortcuts**, **Games**, and **Office** are empty starter folders. They don’t scan your PC or install anything.
2. Click **Shortcuts** to open its Grid Stack. Drag a file, folder, or existing Windows shortcut from Explorer onto an empty cell. Or right-click a cell and pick **Add shortcut here…**.
3. Click the new item to open it. Right-click it and choose **Shortcut properties…** to edit the name, target, or icon.
4. To pin something directly to the dock, close the grid and drop it onto the dock, or right-click the dock and choose **Add shortcut to dock…**.
5. Right-click the collapsed dock and select **Settings…** to change the dock’s position, look, and launch behavior. Export a backup once you have a layout you want to keep.

| Starter folder | Grid size | What it’s useful for |
| --- | --- | --- |
| **Shortcuts** | 5 columns × 3 rows | Everyday apps, downloads, or links |
| **Games** | 6 columns × 3 rows | Game shortcuts, launchers, or game folders |
| **Office** | 4 columns × 3 rows | Docs, spreadsheets, work apps, or project folders |

## Shortcuts and icons

### Add a target manually

Pick a **Display name** and **Target type**, then enter or browse to the **Target location**. Select **Save shortcut** when you’re done.

| Target type | Example | How it opens |
| --- | --- | --- |
| **Program** | `C:\Tools\MyApp\MyApp.exe` | Launches the EXE. Add flags such as `--profile "Work"` in **Arguments**. Set **Start in** to an existing folder if the app needs a specific working directory. If left blank, StackDock uses the EXE’s folder. |
| **File** | `D:\Projects\Notes.txt` | Opens the file in its associated Windows app. Windows `.lnk` and `.url` files can also be added as existing files. |
| **Folder** | `D:\Projects\Screenshots` | Opens the existing disk folder in Windows. This is different from a StackDock folder, which opens an internal Grid Stack. |
| **Web link** | `https://example.com` | Opens through the system’s URL handler. App links such as `steam://rungameid/228300` require a registered handler and a scheme allowed by StackDock’s opener configuration. |

Use full file paths. Don’t paste a complete command line into **Target location**. If a game already has a working Start menu or desktop shortcut, try adding that shortcut first instead of rebuilding its command and arguments by hand.

**Open Destination** opens the target’s location in Explorer where supported. **Open Link** launches a web or app link. **Remove shortcut** removes the dock or grid entry, not the underlying file, app, or folder.

### Choose an icon

1. Right-click the item and open **Shortcut properties…**.
2. Use the browse button beside **Icon path** to choose a PNG, JPEG, GIF, WebP, BMP, or ICO. Resource files such as EXE, DLL, and MUN can also contain embedded icons.
3. If the source has more than one icon, select **Choose embedded icon…**, pick one, then select **Use selected icon**.
4. Use the reset button beside **Icon path** to clear a custom icon and return to the default behavior for that item.

StackDock can read the icon declared by a dropped Windows shortcut if that icon is accessible. For a dropped EXE, it can read artwork embedded in the program. If a `.lnk` does not supply a usable icon, StackDock can try its executable target. A Steam `.url` file can supply an `IconFile` and `IconIndex`, so it can show the game’s artwork rather than a generic icon. Reading artwork does **not** run the executable. If a referenced icon file moves or can’t be read, the item can fall back to a built-in icon.

**Image thumbnails.** On a fresh layout, supported image files use the image itself as their icon. Change this globally in **Settings → General → Behavior → Shortcut icons**, or toggle **Thumbnail** for an individual image shortcut in **Shortcut properties**. A custom icon takes priority. An image format Windows can open isn’t necessarily one StackDock can thumbnail.

**Default artwork.** In builds with separate file icons, ordinary files use the document glyph and images use the image glyph when a thumbnail is off or can’t load. Actual Windows `.lnk` and `.url` files use the shortcut arrow when their own artwork can’t be read. Older builds may show the same arrow for ordinary files.

## Organize the dock and Grid Stacks

### Dock

- **Add a StackDock folder** with **New folder…** in the dock’s right-click menu. Give it a name, choose an icon, and set its grid dimensions.
- **Reorder items** by dragging them along the dock to the insertion marker. You can also right-click an item and use **Move folder earlier/later** or **Move shortcut earlier/later**.
- **Move the dock** with **Position** in the dock menu, or use **Dock settings → Layout → Screen position**. Choose Top, Bottom, Left, or Right. Alignment is along that edge, and orientation can follow the edge or be forced horizontal or vertical.
- **Rename or remove a folder** from its context menu. Removing a StackDock folder also removes the shortcuts and subfolders inside it from the layout. It does not delete their real files.

### Grid Stacks and subfolders

- **Move an item** by dragging it to another cell in the same grid. Dropping on an occupied cell swaps the two entries. Empty cells stay empty unless you place something in them.
- **Drop several items from Explorer** to place them from the drop location across available cells. StackDock skips occupied cells and can add rows as needed, up to the grid limit.
- **Add space** with **Insert row below** or **Insert column right** in the grid context menu. You can also drag an existing tile toward a grid edge and hold for about a second until an extra row or column is offered. Drop into the grid to keep the expansion.
- **Remove space** with **Remove empty row** or **Remove empty column**. These actions only appear for empty rows or columns when more than one remains. To shrink a grid in Settings, first move items out of cells that would be cut off.
- **Create a nested grid** with **New subfolder…** on an empty cell. Click the subfolder to open it. Use the header’s Back arrow or **Esc** to return to the parent. You can give the subfolder its own grid layout and appearance.
- **Close a grid** with **Esc** from the top level, by clicking outside it, or by using **Close grid**. You can also tell StackDock to close the grid when you switch apps.

The Grid Stack editor supports **1–12 columns** and **1–80 rows**. Smaller grids are easier to browse. Use the mouse wheel or touchpad to see offscreen cells when a grid is larger than the available desktop area.

## Customize StackDock

Right-click the collapsed dock and select **Settings…**, or open **Folder settings…** or **Grid Stack settings…** from the item you’re editing. Use the search field at the top of Settings to find a control. Change the folder selector, and **Editing subfolder** when available, to edit the intended grid.

| Tab | What it controls |
| --- | --- |
| **General** | Sign-in startup, switching-app behavior, animation speed, hover names and details, image thumbnails, launch indicator, Settings themes, layout backups, and reset. |
| **Dock** | Screen edge, alignment, icon orientation and size, labels, spacing, padding, panel color, opacity, corners, borders, shadows, folder arrows, and hover effects. |
| **Folder** | The name, built-in or custom icon, and hover label for the selected folder. Top-level folders can override dock icon size, roundness, rotation, or arrow behavior. A subfolder uses its parent grid’s icon size and shape. |
| **Grid Stack** | Rows, columns, optional Settings gear, title icon, shortcut icon size and rotation, label size and width, spacing, padding, panel appearance, hover effects, and entrance animation for the selected folder. |
| **About** | App version, source code, releases, issue tracker, maintainer, and license links. Look here for the running app version, rather than the version printed by npm. |

**Apply** saves regular settings without closing the window. **Done** saves and closes it. **Cancel** discards unapplied changes. Settings themes are saved separately, as explained below.

Drag the title bar to move **Settings** or **Shortcut Properties**, or use the resize grip. While Settings is open, the exposed part of the dock and grid remains usable. Move Settings if it covers the grid. Close Settings before opening another editor, such as **Shortcut properties…**.

### Panel and icon effects

- **Panel appearance:** Set a background color and opacity for the dock or individual grid. Customize its corner radius, border color, thickness, and opacity. Panel transparency is controlled by opacity, not backdrop blur.
- **Spacing:** Use one value for both axes, or enable independent horizontal and vertical values for icon spacing and inner padding.
- **Hover:** Choose a hard or soft highlight, expand the pointer hit area, or add motion such as enlargement, lift, or tilt. Adjust the motion’s timing and easing. Names and delayed target details are separate controls. When captions are already visible, the hover label doesn’t repeat them.
- **Depth:** Reflections sit below icons. Ground shadows have their own intensity and vertical offset. These icon effects are separate from the panel’s shadow.
- **Folder arrows:** Turn on an arrow overlay for dock folders and adjust its placement or rotation. Override it for an individual top-level folder if needed. This overlay is unrelated to the fallback icon for a Windows shortcut.

### Settings themes

Go to **Settings → General → Themes**. Choose a preset such as **Midnight Plum**, **Graphite**, or **High Contrast**, and switch between **Dark** and **Light**. Turn on **Enable custom colors** to change individual roles such as text, surfaces, borders, focus, and buttons.

**Apply Theme** saves the theme. **Discard preview** returns to the last saved theme. The theme styles Settings and Shortcut Properties. It does not recolor the dock or your Grid Stacks, which have independent panel controls.

### Grid entrance animations

Open **Settings → Grid Stack → Entrance** for the folder you want to change. Use **Replay preview** before saving. Animations must be enabled in **General → Behavior**. Reduced-motion preferences can also suppress the preview.

| Style | What you see |
| --- | --- |
| **Classic** | The original Grid Stack reveal |
| **Row Sweep** | Rows enter from top to bottom |
| **Column Sweep** | Columns enter from left to right |
| **Fly In** | Icons glide in from different directions into their cells |
| **Diagonal Wave** | A sweep starts at the upper-left corner |
| **Center Out** | The center opens first, followed by the outside |
| **Outer Ring** | Outer icons appear first, then the middle fills in |
| **All at Once** | Icons fade and scale in together |

### Launch indicator

Go to **Settings → General → Launch indicator** to turn on visual feedback when a shortcut opens. Set how long it stays visible, how the icon glides in, and how quickly the dock returns. You can customize the `Launched` caption, icon and text sizes, panel dimensions, and colors, or have the indicator follow the dock background. This feedback means StackDock handed off the launch. It doesn’t tell you whether the external app finished loading.

## Search, keyboard controls, and startup

| Action | Control |
| --- | --- |
| Find a shortcut | **Ctrl+F** while StackDock has focus, or **Find shortcut…** in the dock context menu |
| Search by scope | Type part of an item name, its folder path, or its target. Search includes dock shortcuts and nested Grid Stacks. |
| Launch a search result | Use **↑** or **↓**, then **Enter**, or click the result. |
| Open the focused item’s context menu | **Shift+F10** or the **Menu** key |
| Dismiss a menu or back out of a grid | **Esc** closes the menu first, goes to the parent from a subfolder, or closes a top-level grid. |
| Quit StackDock | Right-click the collapsed dock and select **StackDock → Exit**. |

**Start at sign-in.** In a release build, open **Settings → General → Behavior → Windows startup**, enable **Start StackDock when I sign in**, and choose **Apply** or **Done**. This setting applies to the current Windows account. Development builds cannot enable startup.

## Back up and restore

StackDock saves settings and layout locally. Export a backup before a reset, an upgrade you want to test, or a move to another PC.

### Export and import

1. To export, choose **Settings → General → Layout Backup → Export layout…**. You can also use **StackDock → Export layout…** from the collapsed dock menu.
2. Save the suggested timestamped JSON file somewhere outside the app folder. Example filename: `stack-dock-layout_2026-11-18_00-16-15.json`.
3. To restore, choose **Import layout…**, select a StackDock JSON export, and confirm **Replace layout**. Import replaces the saved layout rather than merging it.
4. If you moved to another PC, test your shortcuts. Use **Shortcut properties…** to update targets, working directories, or external icon paths that no longer exist.

An export contains the dock and folders, shortcuts, grid positions, appearance and theme settings, and embedded custom icon images where applicable. It can also store references to icon files. It does **not** copy the apps, documents, images used as thumbnail targets, or other files being launched. The native backup dialog accepts StackDock JSON exports up to **8 MiB**. Import does not change Windows sign-in startup.

### Reset to defaults

Open **Settings → General → Layout Backup → Reset to defaults**. Choose **Reset StackDock…**, then hold the confirmation button for **three seconds**. With a keyboard, focus the button and hold **Space** or **Enter**. Releasing early cancels the hold.

Reset removes the current StackDock layout, shortcuts, custom icons, and settings, restores the three starter folders, and turns off Windows sign-in startup. It does not delete your source files or exported backups.

### Privacy and sharing

No account or cloud sync is required. The version-one JSON export is readable text and may contain full Windows paths, URLs, launch arguments, and icon images. Review and redact those fields before sharing an export or a screenshot. Only import layouts you trust and check their targets before launching anything.

## Troubleshooting

| Problem | What to try |
| --- | --- |
| GitHub’s ZIP doesn’t contain `StackDock.exe` | You likely downloaded **Source code**. Look for the binaries under **Releases → Assets**. |
| The app won’t start or its window is blank | Verify the download matches your Windows architecture and that WebView2 is installed. Extract the portable ZIP before running it. If you built from source, try `npm run tauri dev` and check the terminal output. |
| Explorer won’t drop a file onto the dock or grid | Use the native desktop app and drop onto the visible dock or an open grid. Try running StackDock normally rather than as administrator. Close another editor if it’s blocking the target, or add the entry manually. |
| An EXE or file won’t open | Check that the absolute path exists. For a Program shortcut, keep flags in **Arguments** and make sure **Start in** is an existing folder. |
| A Steam or other app link won’t open | Test the same link in Windows first. The matching application must be installed and registered. If Windows can open it but StackDock cannot, report the scheme and error without sharing private details. |
| A shortcut shows an unexpected icon | In **Shortcut properties**, check **Icon path** and the target type. Reset or browse for another icon. For photos, also check the global and per-shortcut thumbnail settings. |
| A plain text file shows the shortcut arrow | Check whether you added the actual `.txt` or a Windows `.lnk` that points to it. Older builds may use the same fallback artwork for ordinary files and shortcut files. You can assign a custom icon in **Shortcut properties**. |
| The dock is clipped, in the wrong place, or obscured | Try another edge or reduce icon size, spacing, or padding. For an issue report, include monitor layout, resolution, Windows display scaling, taskbar position, and auto-hide settings. |
| A change disappears after closing Settings | Use **Apply** or **Done** for regular settings and **Apply Theme** for theme changes. If StackDock reports a storage error, save a backup before closing it. |
| A backup imports but items no longer launch | The backup restores paths and URLs, not their targets. Check drive letters, account folders, app installations, and external icon paths. |
| Release runs but Windows says “Unknown publisher” | The installer’s display manufacturer and the executable’s digital signature are separate. Editing the display name does not sign a release. |

## Developer and release guide

### Run from source

Use a Windows machine to test native behavior. You’ll need Git or a complete source ZIP, a Node.js version supported by the installed Vite release, npm, a current stable Rust **MSVC** toolchain, Microsoft C++ Build Tools with **Desktop development with C++**, and WebView2. The Vite 8 baseline uses Node.js 20.19+ on the 20.x line or 22.12+. A supported Node.js LTS compatible with the repository’s lockfile is a good default. [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) · [Vite requirements](https://vite.dev/guide/)

Clone the repository or extract its source, then open PowerShell in the directory containing `package.json` and `src-tauri`.

```powershell
npm ci
npm run tauri dev
```

Use `npm install` if your source archive has no npm lockfile. The Tauri command starts the development server and opens the desktop app. `npm run dev` alone is useful for browser UI work, but it doesn’t exercise native launching, Explorer file drops, Windows pickers, sign-in startup, or window clipping. If the toolchain fails, gather diagnostics with `npm run tauri info`.

### Source map

| Path | Responsibility |
| --- | --- |
| `index.html` | Main interface and state, dock and Grid Stack rendering, starter folders, icon artwork, settings forms, and grid entrance styles |
| `public/stack-dock-native.js` | Connects the interface to Windows pickers, launch search, icon discovery, file drops, and JSON backup validation |
| `public/stack-dock-widget.js` | Hover effects, launch indicator, drag-to-grow grid behavior, and work-area placement |
| `public/stack-dock-panels.js` | Settings theme presets, customization, and dialog resizing |
| `public/stack-dock-panels.css`, `public/stack-dock-settings-skin.css` | Panel layout and visual styling |
| `src-tauri/src/lib.rs` | Tauri setup, native launch commands, and Windows startup integration |
| `src-tauri/src/native_shortcuts.rs` | Native file dialogs, icon extraction, inspection of dropped targets, and backup file I/O |
| `src-tauri/src/desktop_widget.rs`, `src-tauri/src/file_drop.rs` | Windows-specific dock work-area and file-drop integration |
| `src-tauri/tauri.conf.json`, `src-tauri/capabilities/` | App identity, versions, permissions, window configuration, and installer settings |

When changing defaults, update the starter layout in `fresh()` in `index.html`. Keep legacy icon keys available if old layouts may reference them. Changing defaults should not overwrite an existing user layout. If you add a setting that must survive backup and restore, update the import validation as well as the UI and default value.

### Build Windows installers and the portable ZIP

Choose **one** command from the project root.

| Command | Output |
| --- | --- |
| `npm run tauri build -- --bundles nsis,msi` | Setup EXE and MSI, plus the standalone app binary |
| `npm run tauri build -- --bundles nsis` | Setup EXE and standalone app binary |
| `npm run tauri build -- --bundles msi` | MSI and standalone app binary |
| `npm run tauri build -- --no-bundle` | Standalone app binary without installers |

| Artifact | Default output path |
| --- | --- |
| Setup EXE | `src-tauri/target/release/bundle/nsis/` |
| MSI | `src-tauri/target/release/bundle/msi/` |
| Standalone EXE | `src-tauri/target/release/StackDock.exe` |

For the portable release, copy the **standalone EXE**, not the Setup EXE, into a clean folder, include the repository’s license, and ZIP it. Don’t package `target`, `node_modules`, or the whole project. Test the extracted ZIP on a Windows account or machine where StackDock isn’t installed.

**MSI version and manufacturer.** In `src-tauri/tauri.conf.json`, `bundle.publisher` sets the display manufacturer. For a prerelease app version such as `0.1.0-beta`, configure a numeric MSI version such as `0.1.0` at `bundle.windows.wix.version`. Add the `wix` object **beside** the existing `nsis` object under `bundle.windows`. Leave the top-level app version and other settings intact. If the app binary already compiled and only bundling failed, use `npm run tauri bundle -- --bundles msi` after fixing the configuration. Use `tauri build` instead if the app code or embedded UI changed. If WiX fails at `light.exe`, check the Windows **VBScript** optional feature. [Tauri CLI reference](https://v2.tauri.app/reference/cli/) · [Windows packaging guide](https://v2.tauri.app/distribute/windows-installer/) · [WiX configuration](https://v2.tauri.app/reference/config/#wixconfig)

### Verify a release

- Test a fresh install and confirm the starter folders, version in **About**, and available release formats.
- Test upgrading from the preceding release without losing the layout. Export first, then check that the export can also be restored.
- Launch an EXE, a local file, a folder, an HTTP(S) link, and a Windows shortcut. Check that custom icons, thumbnails, and fallbacks behave as expected.
- Test dock and grid drag-and-drop, subfolder navigation, search, resizing Settings, and launch feedback.
- Check at least one non-default display scaling or screen-edge layout. Test sign-in startup and uninstall behavior with a release build.
- In the release notes, list changes and known issues. Upload the tested binaries and label each asset with its architecture and package type. Keep personal layouts, logs, and signing credentials out of the release.

## Support and license

Report bugs through the [GitHub issue tracker](https://github.com/TNVR-HSN/stack-dock/issues). Include the StackDock version from **Settings → About**, Windows version, install type, and the shortest steps that reproduce the issue. Describe what you expected and what happened. For display issues, add monitor and scaling details. For launcher issues, give the target type and the error message with private paths or URLs removed.

For a pull request, describe the behavior you changed and how you tested it. A before-and-after capture helps for dock, grid, or animation changes. Changes to native behavior should be tested in the Windows desktop app, not only in a browser.

StackDock is distributed under the [MIT License](https://opensource.org/license/mit). The repository’s `LICENSE` file contains its copyright and license notice. Third-party components have their own licenses.
