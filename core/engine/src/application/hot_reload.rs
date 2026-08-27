use crate::application::ports::{FileWatchPort, RuntimeEngine};
use crate::domain::error::EngineError;
use crate::domain::hot_reload::{DebounceDuration, HotReloadStatus};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

struct SlotData<AST> {
    ast: Arc<AST>,
    version: u64,
}

/// Thread-safe container managing the atomic pointer swap of active compiled scripts (ADR-0005, C-11).
pub struct AtomicScriptSlot<AST> {
    inner: Arc<RwLock<Option<SlotData<AST>>>>,
}

impl<AST> Default for AtomicScriptSlot<AST> {
    fn default() -> Self {
        Self::new()
    }
}

impl<AST> Clone for AtomicScriptSlot<AST> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<AST> AtomicScriptSlot<AST> {
    /// Creates an empty script slot.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    /// Accesses the currently active compiled AST.
    #[must_use]
    pub fn active_ast(&self) -> Option<Arc<AST>> {
        self.inner
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|d| Arc::clone(&d.ast)))
    }

    /// Returns the monotonic version counter of the active script.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.inner
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|d| d.version))
            .unwrap_or(0)
    }

    /// Atomically replaces the active AST with a newly compiled one, returning the new version.
    pub fn swap(&self, new_ast: AST) -> u64 {
        let mut guard = self.inner.write().unwrap();
        let next_version = guard.as_ref().map(|d| d.version + 1).unwrap_or(1);
        *guard = Some(SlotData {
            ast: Arc::new(new_ast),
            version: next_version,
        });
        next_version
    }
}

/// Coordinates background compilation, atomic pointer swapping, and error rollback (PRD-004, C-11, C-12).
pub struct HotReloadCoordinator<E: RuntimeEngine> {
    engine: Arc<E>,
    slot: AtomicScriptSlot<E::CompiledScript>,
}

impl<E: RuntimeEngine> HotReloadCoordinator<E> {
    /// Creates a new `HotReloadCoordinator`.
    #[must_use]
    pub const fn new(engine: Arc<E>, slot: AtomicScriptSlot<E::CompiledScript>) -> Self {
        Self { engine, slot }
    }

    /// Accesses the underlying script slot.
    #[must_use]
    pub const fn slot(&self) -> &AtomicScriptSlot<E::CompiledScript> {
        &self.slot
    }

    /// Compiles the script and swaps the active AST atomically (C-11, C-12).
    ///
    /// If compilation fails, the previous active AST is retained and diagnostics are returned.
    pub fn compile_and_swap(&self, script_source: &str) -> HotReloadStatus {
        match self.engine.compile(script_source) {
            Ok(new_ast) => {
                let version = self.slot.swap(new_ast);
                HotReloadStatus::Success { version }
            }
            Err(err) => {
                // Keep existing AST and report syntax/compilation error diagnostic (C-12)
                HotReloadStatus::CompilationError {
                    error: err.to_string(),
                    previous_version: self.slot.version(),
                }
            }
        }
    }
}

/// Filesystem watcher monitoring script files with debouncing using a `FileWatchPort` (PRD-004, C-10, C-29).
pub struct ScriptWatcher<W: FileWatchPort> {
    debounce: DebounceDuration,
    watcher: W,
}

impl<W: FileWatchPort> ScriptWatcher<W> {
    /// Creates a new `ScriptWatcher` parameterized with a `FileWatchPort` adapter.
    #[must_use]
    pub const fn new(debounce: DebounceDuration, watcher: W) -> Self {
        Self { debounce, watcher }
    }

    /// Creates a new `ScriptWatcher` with a custom `FileWatchPort` implementation.
    pub const fn with_watcher(debounce: DebounceDuration, watcher: W) -> Self {
        Self::new(debounce, watcher)
    }

    /// Starts watching a path for `.rhai` modifications with debouncing.
    ///
    /// # Errors
    /// Returns `EngineError` if watcher initialization fails.
    pub fn watch<F>(&mut self, path: &Path, on_change: F) -> Result<(), EngineError>
    where
        F: Fn(PathBuf) + Send + Sync + 'static,
    {
        let debounce_dur = self.debounce.as_duration();
        let last_trigger = Arc::new(Mutex::new(Instant::now() - debounce_dur));
        let callback = Arc::new(on_change);

        self.watcher.watch(
            path,
            Box::new(move |file_path: PathBuf| {
                if file_path.extension().is_none_or(|ext| ext != "rhai") {
                    return;
                }

                let mut last = last_trigger.lock().unwrap();
                let now = Instant::now();
                if now.duration_since(*last) >= debounce_dur {
                    *last = now;
                    callback(file_path);
                }
            }),
        )
    }
}
