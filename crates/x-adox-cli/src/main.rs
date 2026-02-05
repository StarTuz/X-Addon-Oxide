// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use x_adox_core::scenery::{SceneryManager, SceneryPackType};
use x_adox_core::XPlaneManager;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to X-Plane root
    #[arg(short, long, env = "XPLANE_ROOT")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all scenery packs
    List,
    /// Enable a scenery pack by name (partial match)
    Enable { name: String },
    /// Disable a scenery pack by name (partial match)
    Disable { name: String },
    /// Run Smart Sort with duplicate detection
    SmartSort,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = match cli.root {
        Some(path) => path,
        None => XPlaneManager::try_find_root().ok_or_else(|| {
            anyhow::anyhow!("Could not find X-Plane root. Please specify with --root.")
        })?,
    };

    let manager = XPlaneManager::new(&root)?;
    let mut scenery = SceneryManager::new(manager.get_scenery_packs_path());
    scenery.load()?;

    match &cli.command {
        Commands::List => {
            println!("Listing Scenery Packs in {:?}", root);
            for pack in &scenery.packs {
                let status = match pack.status {
                    SceneryPackType::Active => "[x]",
                    SceneryPackType::Disabled => "[ ]",
                    SceneryPackType::DuplicateHidden => "[D]",
                };
                println!("{} {}", status, pack.name);
            }
        }
        Commands::Enable { name } => {
            let mut found = false;
            for pack in &mut scenery.packs {
                if pack.name.contains(name) {
                    pack.status = SceneryPackType::Active;
                    println!("Enabled: {}", pack.name);
                    found = true;
                }
            }
            if found {
                scenery.save(None)?;
            } else {
                println!("No package found matching '{}'", name);
            }
        }
        Commands::Disable { name } => {
            let mut found = false;
            for pack in &mut scenery.packs {
                if pack.name.contains(name) {
                    pack.status = SceneryPackType::Disabled;
                    println!("Disabled: {}", pack.name);
                    found = true;
                }
            }
            if found {
                scenery.save(None)?;
            } else {
                println!("No package found matching '{}'", name);
            }
        }
        Commands::SmartSort => {
            println!("Running Smart Sort on {:?}", root);

            let model = x_adox_bitnet::BitNetModel::new().unwrap_or_default();
            let context = x_adox_bitnet::PredictContext::default();
            scenery.sort(Some(&model), &context);
            scenery.save(Some(&model))?;
            println!("Smart Sort complete. Duplicates disabled and file reordered.");
        }
    }

    Ok(())
}
