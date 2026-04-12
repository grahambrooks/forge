//! Path-prefix index mapping source directories to container ids.
//!
//! Scanners register a container against the directory it was discovered in
//! (typically the directory containing the manifest file). When a later scanner
//! encounters a source file, it looks up the deepest matching prefix to decide
//! which container owns that file. This replaces the previous fuzzy
//! slug-contains heuristic in `source.rs` and handles monorepos correctly.

use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct ContainerIndex {
    entries: Vec<(PathBuf, String)>,
}

impl ContainerIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `container_id` as owning everything under `source_dir`.
    pub fn register(&mut self, source_dir: PathBuf, container_id: String) {
        let canonical = source_dir.canonicalize().unwrap_or(source_dir);
        if !self
            .entries
            .iter()
            .any(|(p, id)| p == &canonical && id == &container_id)
        {
            self.entries.push((canonical, container_id));
        }
    }

    /// Return the container id owning `file`, picking the deepest matching
    /// registered directory.
    pub fn attribute(&self, file: &Path) -> Option<&str> {
        let target = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
        self.entries
            .iter()
            .filter(|(dir, _)| target.starts_with(dir))
            .max_by_key(|(dir, _)| dir.components().count())
            .map(|(_, id)| id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn deepest_prefix_wins() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        let api = root.join("services/api");
        let web = root.join("services/api/web");
        fs::create_dir_all(&web).unwrap();
        fs::write(api.join("marker"), "").unwrap();
        fs::write(web.join("marker"), "").unwrap();

        let mut idx = ContainerIndex::new();
        idx.register(root.to_path_buf(), "root".into());
        idx.register(api.clone(), "api".into());
        idx.register(web.clone(), "web".into());

        let file = web.join("src/app.ts");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "").unwrap();
        assert_eq!(idx.attribute(&file), Some("web"));

        let file2 = api.join("src/main.rs");
        fs::create_dir_all(file2.parent().unwrap()).unwrap();
        fs::write(&file2, "").unwrap();
        assert_eq!(idx.attribute(&file2), Some("api"));
    }

    #[test]
    fn unmatched_returns_none() {
        let tmp = tempdir().unwrap();
        let idx = ContainerIndex::new();
        let file = tmp.path().join("foo.rs");
        fs::write(&file, "").unwrap();
        assert!(idx.attribute(&file).is_none());
    }
}
