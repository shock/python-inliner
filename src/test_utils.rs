use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct MockFileSystem {
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        MockFileSystem {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        let mut files = self.files.lock().unwrap();
        files.insert(path.as_ref().to_path_buf(), content.to_string());
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        let files = self.files.lock().unwrap();
        files.get(path.as_ref()).cloned()
    }

    pub fn write_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        let mut files = self.files.lock().unwrap();
        files.insert(path.as_ref().to_path_buf(), content.to_string());
    }

    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path.as_ref())
    }
}

pub fn setup_test_env() -> MockFileSystem {
    MockFileSystem::new()
}
