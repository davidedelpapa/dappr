use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// StyleStorage is a simple storage for CSS content.
//
// Can load and reset a file at compile time.
// The most iportant function is that it can discharge the data,
// thereby shrinking the size of the final WASM output
#[doc(hidden)]
#[allow(dead_code)]
pub struct StyleStorage {
    inner: Mutex<Option<String>>,
}

impl StyleStorage {
    // Creates a new instance with the given text content.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn new(css: &str) -> Self {
        Self {
            inner: Mutex::new(Some(css.to_string())),
        }
    }

    // Returns a clone of the string wrapped in `Option<String>`, preserving `None` state.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn get(&self) -> Option<String> {
        self.inner
            .lock()
            .expect("Cannot get a lock on the style resource")
            .clone()
    }

    // Returns the content or an empty string if `None` (fast access).
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn get_or_empty(&self) -> String {
        self.inner
            .lock()
            .expect("Could not get a lock on the style resource")
            .clone()
            .unwrap_or_else(|| "".to_string())
    }

    // Clears the stored string, freeing memory.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn set_none(&self) {
        *self
            .inner
            .lock()
            .expect("Could not get a lock on the style resource") = None;
    }

    // Resets the storage with new content.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn reset(&self, css: &str) {
        *self
            .inner
            .lock()
            .expect("Could not get a lock on the style resource") = Some(css.to_string());
    }
}

// Temp dir, deletes when dropped. Used mainly in tests.
// It has two paths, one that will be deleted and one that will be kept if not empty
// at the end of the test.
#[doc(hidden)]
pub struct TempDir {
    path: PathBuf,
    path_may_keep: PathBuf,
}

impl TempDir {
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn new(path: &str) -> Self {
        fs::create_dir_all(path).expect("Failed to create temporary directory");
        TempDir {
            path: PathBuf::from(path),
            path_may_keep: PathBuf::new(),
        }
    }
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn may_keep(path: &str) -> Self {
        fs::create_dir_all(path).expect("Failed to create temporary directory");
        TempDir {
            path: PathBuf::new(),
            path_may_keep: PathBuf::from(path),
        }
    }
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn get_may_keep_path(&self) -> &PathBuf {
        &self.path_may_keep
    }
}

impl Drop for TempDir {
    #[doc(hidden)]
    fn drop(&mut self) {
        // Clean up the directory when the struct goes out of scope
        if self.path.exists() {
            fs::remove_dir_all(&self.path).expect("Failed to remove temporary directory");
        }
        if self.path_may_keep.exists() {
            let _ = fs::remove_dir(&self.path_may_keep);
        }
    }
}
