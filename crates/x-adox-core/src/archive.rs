// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchiveEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Suggested recommendation for extraction based on classification
    pub recommended: bool,
}

pub struct UnifiedArchiveReader;

impl UnifiedArchiveReader {
    pub fn list_contents(path: &Path) -> Result<Vec<ArchiveEntry>> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .ok_or_else(|| anyhow!("No file extension found"))?;

        match extension.as_str() {
            "zip" => Self::list_zip(path),
            "7z" => Self::list_7z(path),
            "rar" => Self::list_rar(path),
            _ => Err(anyhow!("Unsupported archive format: {}", extension)),
        }
    }

    fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut entries = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            entries.push(ArchiveEntry {
                path: file.name().to_string(),
                is_dir: file.is_dir(),
                size: file.size(),
                recommended: Self::is_recommended_path(file.name()),
            });
        }

        Ok(entries)
    }

    fn list_7z(path: &Path) -> Result<Vec<ArchiveEntry>> {
        let mut entries = Vec::new();
        let reader = sevenz_rust2::SevenZReader::open(path, sevenz_rust2::Password::empty())
            .map_err(|e| anyhow!("Failed to open 7z: {}", e))?;

        for entry in &reader.archive().files {
            entries.push(ArchiveEntry {
                path: entry.name().to_string(),
                is_dir: entry.is_directory(),
                size: entry.size(),
                recommended: Self::is_recommended_path(entry.name()),
            });
        }
        Ok(entries)
    }

    fn list_rar(path: &Path) -> Result<Vec<ArchiveEntry>> {
        let mut entries = Vec::new();
        let mut open_archive = unrar::Archive::new(path)
            .open_for_listing()
            .map_err(|e| anyhow!("Failed to open RAR: {:?}", e))?;

        while let Some(header) = open_archive.next() {
            let header = header.map_err(|e| anyhow!("RAR entry error: {:?}", e))?;
            let name = header.filename.to_string_lossy().to_string();
            entries.push(ArchiveEntry {
                path: name.clone(),
                is_dir: header.is_directory(),
                size: header.unpacked_size as u64,
                recommended: Self::is_recommended_path(&name),
            });
        }

        Ok(entries)
    }

    fn is_recommended_path(path: &str) -> bool {
        let lower = path.to_lowercase();

        // Exclude junk
        if lower.contains("__macosx")
            || lower.contains(".ds_store")
            || lower.contains("thumbs.db")
            || lower.ends_with(".zip")
            || lower.ends_with(".7z")
            || lower.ends_with(".rar")
        {
            return false;
        }

        // Recommend core addon files/folders
        if lower.ends_with("apt.dat")
            || lower.ends_with(".dsf")
            || lower.ends_with(".acf")
            || lower.ends_with(".xpl")
        {
            return true;
        }

        // Folders that often represent the root of an addon
        let parts: Vec<&str> = lower.split('/').collect();
        for part in parts {
            if part == "scripts"
                || part == "pythonplugins"
                || part == "objects"
                || part == "earth nav data"
            {
                return true;
            }
        }

        // By default, recommend most things unless they look like top-level readme/license strings
        if lower.ends_with("readme.txt")
            || lower.ends_with("license.txt")
            || lower.ends_with("instructions.txt")
        {
            return false;
        }

        // If it's a directory and looks like an ICAO or brand name, it might be a good root.
        // But for now, let's keep it simple: recommend unless it's junk.
        true
    }
}
