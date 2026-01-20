# Aircraft Preview Fallbacks

X-Addon-Oxide provides a robust aircraft preview system. When a specific icon for an aircraft (like a CSL or AI model) is missing, the application automatically searches for related icons and provides high-quality category-based fallbacks.

## How it Works

When you select an aircraft, the application performs the following search:

1. **Exact Match**: Looks in the aircraft's folder for `{acf_name}_icon11.png` or `icon11.png`.
2. **Parent Folder Fallback**: If not found, it checks the parent directory. This is particularly useful for CSL packages where multiple aircraft models might share a single icon in a common folder.
3. **Heuristic Match**: It scans the folder for any PNG files containing keywords like `icon`, `preview`, or `thumbnail`.
4. **Category Fallback**: If all else fails, it uses the aircraft's AI-determined category (Airliner, General Aviation, Military, or Helicopter) to display a premium bundled placeholder.

## Customizing Fallback Icons

If you wish to change the default placeholder images used by the application, follow these steps:

1. **Locate the Assets**: The bundled fallback images are located in the source code at:
    `crates/x-adox-gui/assets/`
    - `fallback_airliner.png`
    - `fallback_ga.png`
    - `fallback_military.png`
    - `fallback_helicopter.png`

2. **Replace the Files**: Overwrite these files with your own PNG images. For best results, use a **4:3 aspect ratio** and a resolution of at least **512x384**.

3. **Rebuild the Application**: Since these assets are embedded into the binary using `include_bytes!`, you must recompile the application for the changes to take effect:

    ```bash
    cargo build --release -p x-adox-gui
    ```

## Adding Specific Icons for Addons

To avoid fallbacks altogether for a specific aircraft:

1. Open the folder containing the aircraft's `.acf` file.
2. Add a PNG image named `icon11.png`.
3. X-Addon-Oxide will automatically pick this up on next selection.
