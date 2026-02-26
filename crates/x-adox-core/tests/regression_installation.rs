// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use x_adox_core::management::{ArchiveType, ModManager};

#[test]
fn test_detect_archive_type() {
    // 1. Standard Plugin
    let plugin_files = vec![
        "MyPlugin/lin_x64/plugin.xpl".to_string(),
        "MyPlugin/data/config.txt".to_string(),
    ];
    assert_eq!(
        ModManager::detect_archive_type(&plugin_files),
        ArchiveType::StandardPlugin
    );

    // 2. Lua Script (Flat)
    let lua_files = vec!["bushtalk.lua".to_string()];
    assert_eq!(
        ModManager::detect_archive_type(&lua_files),
        ArchiveType::LuaScripts
    );

    // 3. Lua Script (Nested)
    let lua_nested = vec!["FlyWithLua/Scripts/myscript.lua".to_string()];
    assert_eq!(
        ModManager::detect_archive_type(&lua_nested),
        ArchiveType::LuaScripts
    );

    // 4. Python Script
    let py_files = vec!["myscript.py".to_string()];
    assert_eq!(
        ModManager::detect_archive_type(&py_files),
        ArchiveType::PythonScripts
    );

    // 5. Mixed but has binary
    let mixed = vec!["plugin.xpl".to_string(), "utility.lua".to_string()];
    assert_eq!(
        ModManager::detect_archive_type(&mixed),
        ArchiveType::StandardPlugin
    );

    // 6. Unknown
    let unknown = vec!["readme.txt".to_string()];
    assert_eq!(
        ModManager::detect_archive_type(&unknown),
        ArchiveType::Unknown
    );
}
