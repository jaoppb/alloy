//! Multi-version XDG isolation, script resolution shadowing, and live origin syncing (PRD-004, C-10, C-11, C-12, C-25).

use crate::error::AlloyCliError;
use std::path::{Path, PathBuf};

pub const VERSION_FINGERPRINT: &str = env!("CARGO_PKG_VERSION");

/// Manages XDG script directories with multi-version isolation and resolution shadowing.
pub struct XdgScriptManager {
    custom_scripts_dir: Option<PathBuf>,
    data_version_dir: PathBuf,
    config_version_dir: PathBuf,
    config_dir: PathBuf,
    base_data_dir: PathBuf,
}

impl XdgScriptManager {
    /// Initializes the XDG manager, resolving paths according to XDG Base Directory specification.
    ///
    /// # Errors
    /// Returns `AlloyCliError::Io` if base directory creation fails.
    pub fn new(custom_scripts_dir: Option<PathBuf>) -> Result<Self, AlloyCliError> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        let xdg_data = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".local/share"));
        let base_data_dir = xdg_data.join("alloy");
        let data_version_dir = base_data_dir.join("versions").join(VERSION_FINGERPRINT);

        let xdg_config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));
        let config_dir = xdg_config.join("alloy");
        let config_version_dir = config_dir.join("versions").join(VERSION_FINGERPRINT);

        // Ensure directories exist
        std::fs::create_dir_all(&data_version_dir)?;

        // Update 'current' symlink on Unix
        #[cfg(unix)]
        {
            let symlink_path = base_data_dir.join("current");
            let target = Path::new("versions").join(VERSION_FINGERPRINT);
            let tmp_symlink = base_data_dir.join(format!("current.tmp.{}", std::process::id()));
            let _ = std::fs::remove_file(&tmp_symlink);
            if std::os::unix::fs::symlink(&target, &tmp_symlink).is_ok() {
                let _ = std::fs::rename(&tmp_symlink, &symlink_path);
            }
        }

        Ok(Self {
            custom_scripts_dir,
            data_version_dir,
            config_version_dir,
            config_dir,
            base_data_dir,
        })
    }

    /// Seeds embedded default scripts into `$XDG_DATA_HOME/alloy/versions/<version>/` atomically.
    ///
    /// # Errors
    /// Returns `AlloyCliError::Io` if file writing fails.
    pub fn seed_scripts(&self) -> Result<(), AlloyCliError> {
        let default_scripts: &[(&str, &str)] = &[
            ("pipeline.rhai", crate::DEFAULT_PIPELINE_SCRIPT),
            ("cascade.rhai", css::DEFAULT_CASCADE_SCRIPT),
            ("layout.rhai", graphics::DEFAULT_LAYOUT_SCRIPT),
        ];

        for (filename, content) in default_scripts {
            let target_path = self.data_version_dir.join(filename);
            if !target_path.exists() {
                Self::atomic_write(&target_path, content)?;
                tracing::info!(target: "alloy::xdg", "Seeded default script to {:?}", target_path);
            }
        }

        Ok(())
    }

    /// Resolves a script's content following the strict shadowing hierarchy:
    /// 1. `--scripts-dir <DIR>/<name>`
    /// 2. `$XDG_CONFIG_HOME/alloy/versions/<version>/<name>`
    /// 3. `$XDG_CONFIG_HOME/alloy/<name>`
    /// 4. `$XDG_DATA_HOME/alloy/versions/<version>/<name>`
    /// 5. In-memory embedded fallback
    #[must_use]
    pub fn resolve_script(&self, name: &str, embedded_fallback: &'static str) -> String {
        // 1. Custom CLI scripts-dir
        if let Some(ref custom_dir) = self.custom_scripts_dir {
            let p = custom_dir.join(name);
            if let Ok(content) = std::fs::read_to_string(&p) {
                tracing::debug!(target: "alloy::xdg", "Resolved {} from custom CLI dir: {:?}", name, p);
                return content;
            }
        }

        // 2. Versioned config override
        let p_cfg_ver = self.config_version_dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&p_cfg_ver) {
            tracing::debug!(target: "alloy::xdg", "Resolved {} from versioned config: {:?}", name, p_cfg_ver);
            return content;
        }

        // 3. Unversioned config override
        let p_cfg = self.config_dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&p_cfg) {
            tracing::debug!(target: "alloy::xdg", "Resolved {} from user config: {:?}", name, p_cfg);
            return content;
        }

        // 4. Versioned local share data
        let p_data = self.data_version_dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&p_data) {
            tracing::debug!(target: "alloy::xdg", "Resolved {} from versioned data: {:?}", name, p_data);
            return content;
        }

        // 5. Embedded fallback
        tracing::debug!(target: "alloy::xdg", "Resolved {} from embedded fallback", name);
        embedded_fallback.to_string()
    }

    /// Automatically syncs a modified crate origin script to the versioned XDG data folder.
    ///
    /// # Errors
    /// Returns `AlloyCliError::Io` on write failure.
    pub fn sync_origin_to_data(&self, filename: &str, content: &str) -> Result<(), AlloyCliError> {
        let target_path = self.data_version_dir.join(filename);
        Self::atomic_write(&target_path, content)?;
        tracing::info!(target: "alloy::xdg", "Auto-synced modified origin script to {:?}", target_path);
        Ok(())
    }

    /// Discovers all paths to monitor for hot-reloading (crate origin folders and XDG directories).
    #[must_use]
    pub fn discover_watch_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Monitored XDG directories
        paths.push(self.data_version_dir.clone());
        if self.config_version_dir.exists() {
            paths.push(self.config_version_dir.clone());
        }
        if self.config_dir.exists() {
            paths.push(self.config_dir.clone());
        }
        if let Some(ref custom_dir) = self.custom_scripts_dir {
            if custom_dir.exists() {
                paths.push(custom_dir.clone());
            }
        }

        // 2. Crate origin source folders (development / pair-programming mode)
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let possible_origins = [
            cwd.join("alloy/src"),
            cwd.join("core/css/src/application"),
            cwd.join("core/graphics/src/domain"),
        ];

        for origin in possible_origins {
            if origin.exists() {
                tracing::info!(target: "alloy::xdg", "Found crate origin folder to watch: {:?}", origin);
                paths.push(origin);
            }
        }

        paths
    }

    /// Atomically writes content to `path` via a temporary PID file and rename.
    pub fn atomic_write(path: &Path, content: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pid = std::process::id();
        let tmp_path = path.with_extension(format!("tmp.{pid}"));
        std::fs::write(&tmp_path, content)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    #[must_use]
    pub fn data_version_dir(&self) -> &Path {
        &self.data_version_dir
    }

    #[must_use]
    pub fn base_data_dir(&self) -> &Path {
        &self.base_data_dir
    }
}
