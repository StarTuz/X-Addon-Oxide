// SPDX-License-Identifier: MIT
// Copyright (c) 2026 StarTuz

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

static CONFIG_ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn config_env_mutex() -> &'static Mutex<()> {
    CONFIG_ENV_MUTEX.get_or_init(|| Mutex::new(()))
}

pub struct ScopedConfigRoot {
    _lock: MutexGuard<'static, ()>,
    previous: Option<String>,
}

impl ScopedConfigRoot {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let lock = config_env_mutex().lock().unwrap();
        let path: PathBuf = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();

        let previous = std::env::var("X_ADOX_CONFIG_DIR").ok();
        std::env::set_var("X_ADOX_CONFIG_DIR", &path);

        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for ScopedConfigRoot {
    fn drop(&mut self) {
        if let Some(prev) = &self.previous {
            std::env::set_var("X_ADOX_CONFIG_DIR", prev);
        } else {
            std::env::remove_var("X_ADOX_CONFIG_DIR");
        }
    }
}
