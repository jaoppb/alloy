use crate::application::ports::FileWatchPort;
use crate::domain::error::EngineError;
use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::{Path, PathBuf};

/// Concrete adapter implementing `FileWatchPort` using the `notify` crate (C-29).
pub struct NotifyFileWatcher {
    watcher: Option<notify::RecommendedWatcher>,
}

impl Default for NotifyFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl NotifyFileWatcher {
    /// Creates a new `NotifyFileWatcher`.
    #[must_use]
    pub const fn new() -> Self {
        Self { watcher: None }
    }
}

impl FileWatchPort for NotifyFileWatcher {
    fn watch(
        &mut self,
        path: &Path,
        callback: Box<dyn Fn(PathBuf) + Send + Sync + 'static>,
    ) -> Result<(), EngineError> {
        let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
            if let Ok(event) = res {
                if !event.kind.is_modify() && !event.kind.is_create() {
                    return;
                }

                for p in event.paths {
                    callback(p);
                }
            }
        })
        .map_err(|e| EngineError::RuntimeError(e.to_string()))?;

        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(path, mode)
            .map_err(|e| EngineError::RuntimeError(e.to_string()))?;
        self.watcher = Some(watcher);
        Ok(())
    }
}
