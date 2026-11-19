# StackDock native compatibility preview

This native preview reuses StackDock's layout format and built-in artwork. The normal Tauri executable and installer are unchanged. With the optional Settings connection enabled, the native dock starts WebView2 only when you choose **Settings…**.

## Connect to your Phase2 Settings

Use the local `Phase2` build that already supports `--settings-only` and the shared layout file. Build it without making an installer. From the project root in PowerShell:

```powershell
Remove-Item Env:STACKDOCK_SETTINGS_ONLY -ErrorAction SilentlyContinue
npm run tauri build -- --no-bundle
```

Exit the normal dock and any older native preview. Use the EXE path the build printed. It is normally:

```powershell
$settingsExe = (Resolve-Path .\src-tauri\target\release\StackDock.exe).Path
cargo run --locked --release --manifest-path .\native-prototype\Cargo.toml -- --settings-exe "$settingsExe"
```

Right-click the native dock and select **Settings…**. Change the dock edge, then select **Done**. The Settings window should close and the native dock should move without restarting. **Apply** saves inside the Settings window, but the native preview refreshes only when the window exits. Use **Cancel** to discard unapplied edits. Test the two modes one at a time.

This mode reads `%LOCALAPPDATA%\com.stackdock.launcher\layout.json` directly, rather than an export copied next to the EXE. It expects the standard StackDock application identifier and a helper built from your Phase2 code. If the shared file is missing, start your normal Phase2 dock once, save its Settings, then try again. To return to your normal app, use **Exit native prototype** and run StackDock as usual.

## Open your actual StackDock layout

1. In your normal StackDock, use **Settings → General → Layout Backup → Export layout…** to save a copy of your current layout.
2. Extract the preview ZIP. Rename the exported copy `stack-dock-layout.json` and put it beside `stack-dock-native.exe`. Alternatively, drag an export onto the EXE or pass it on the command line.

   ```powershell
   .\stack-dock-native.exe --layout "C:\path\to\your-export.json"
   ```

3. Exit the normal StackDock before starting the native preview. Click a real dock folder to open it or click one of your shortcuts to launch it. Backspace or **Esc**, or the back button in a subfolder, returns to the parent. Use the mouse wheel to scroll an overflowing grid. The right-click menu exits the preview.

If you do not provide an export, the preview starts with the exact **empty starter layout** from `main`. It will not invent shortcuts. A sample populated layout is in the ZIP for quick testing. It is separate from the default and from your data.

## What is connected to the project

- **The layout is the project’s real version-one backup format.** Saved dock order, direct shortcuts, named and nested stacks, sparse row/column positions, panel colors/opacity, spacing, icon sizes, and dock edge/orientation/alignment come from the export.
- **The artwork is the project’s actual artwork.** `tools/sync-assets.cjs` rasterizes the existing built-in SVGs from the frontend and exports its actual starter defaults. The native renderer also reads embedded custom icons, supported image thumbnails, and locally referenced icon resources. The native resource reader is based on StackDock’s existing Windows icon extraction code.
- **The animation uses the existing dock-to-title behavior.** The folder icon travels into the grid title while the panel expands. Opening and closing use the original timings and easing curves, with the exported animation speed. The visual implementation is native and not pixel-identical to the web version. Nested navigation is immediate. The native Classic, Row/Column Sweep, Fly In, Diagonal, and Outer Ring behaviors are approximations, and Center Out and All at Once do not have their own native implementations yet.
- **Read-only export mode.** Opening a JSON snapshot with `--layout` does not change your saved layout or launch any of its shortcuts. Launches happen when you click an item, so only test trusted layouts.

Without `--settings-exe`, the preview remains a read-only layout viewer that can launch shortcuts. With the flag, the normal Settings code edits the shared layout in a separate, temporary process. The native preview still does not include the complete shortcut editor, hover, arrow, launch-indicator, drag-and-drop, or accessibility behavior. Do not replace your installed build with this EXE.

## Build and test

On Windows x64, from the repository root:

```powershell
cargo test --locked --manifest-path .\native-prototype\Cargo.toml
cargo run --locked --release --manifest-path .\native-prototype\Cargo.toml
cargo run --locked --release --manifest-path .\native-prototype\Cargo.toml -- --smoke-test
```

The Windows smoke test presents closed, opening, and open frames and compares the planned sizes with the actual native window. It does not launch shortcuts. On Linux, `cargo run --manifest-path native-prototype/Cargo.toml -- --render-preview DIRECTORY` writes renderer frames without running the Windows window. Pass `--layout` to render a saved backup.

For a memory comparison, run only one version at a time and use the same backup in both. After a minute with the dock collapsed, record CPU and active private working set. Then check the first open after sitting idle and repeat with a populated grid and subfolders. The native preview still lacks features, so do not treat that measurement as the final product comparison.

## Add to your local branch

Extract only the `native-prototype/` folder from the source ZIP and place it beside `src-tauri/` in your local Phase2 checkout. If you already have an older experiment, move that older folder out of the checkout first. No existing app source or startup settings are replaced.
