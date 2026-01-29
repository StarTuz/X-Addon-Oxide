use crate::scenery::SceneryDescriptor;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Minimal DSF parser that peeks into the string table to identify scenery type.
/// NOTE: Most DSFs are internally compressed, so this only works for uncompressed DSFs.
/// For compressed DSFs, we return a default descriptor (no info) to avoid blocking I/O.
pub struct DsfPeek;

impl DsfPeek {
    pub fn analyze(path: &Path) -> std::io::Result<SceneryDescriptor> {
        let mut file = File::open(path)?;
        let mut descriptor = SceneryDescriptor::default();

        // 1. Check Header (12 bytes: 8 byte signature + 4 byte version)
        let mut header = [0u8; 12];
        file.read_exact(&mut header)?;

        // DSF Signature is "XPLNEDSF" (8 bytes)
        if &header[0..8] != b"XPLNEDSF" {
            // Not a standard DSF - might be 7-zipped or other format
            // Bail out fast to avoid blocking I/O
            return Ok(descriptor);
        }

        // Check if the next atom is "DAEH" (header) or "NFED" (deflate compressed)
        // If DEFLATE compressed, bail out - we can't scan strings.
        let mut atom_id = [0u8; 4];
        if file.read_exact(&mut atom_id).is_err() {
            return Ok(descriptor);
        }

        // "NFED" = "DEFN" reversed = Deflate compressed
        // "SCFG" or "DAEH" = uncompressed
        if &atom_id == b"NFED" || &atom_id == b"7z\xBC\xAF" {
            // Compressed DSF - cannot peek, return empty to avoid slow read
            return Ok(descriptor);
        }

        // 2. Read a small buffer for string scanning (4KB is enough for header atoms)
        let mut buffer = vec![0u8; 4096];
        let bytes_read = file.read(&mut buffer)?;
        let data = &buffer[..bytes_read];

        // Search for library path archetypes
        let content = String::from_utf8_lossy(data);

        // Count occurrences of object-like strings
        descriptor.object_count = content.matches(".obj").count();
        descriptor.facade_count = content.matches(".fac").count();
        descriptor.forest_count = content.matches(".for").count();
        descriptor.polygon_count = content.matches(".pol").count();
        descriptor.mesh_count = content.matches("PATCH").count();

        // Specific airport clues
        if content.contains("lib/airport/") || content.contains("PROPERTY apt_id") {
            descriptor.has_airport_properties = true;
        }

        // Collect some library names for refined classification
        let libraries = ["simheaven", "orbx", "opensceneryxt", "misterx"];
        for lib in libraries {
            if content.contains(lib) {
                descriptor.library_refs.push(lib.to_string());
            }
        }

        Ok(descriptor)
    }
}
