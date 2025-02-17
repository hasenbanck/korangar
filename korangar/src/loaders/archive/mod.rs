pub mod folder;
pub mod native;

use std::path::Path;

pub trait Archive: Send + Sync {
    fn from_path(path: &Path, hash_content: bool) -> Self
    where
        Self: Sized;

    /// Retrieve an asset from the Archive.
    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>>;

    /// Get a list of all files with a given extension.
    fn get_files_with_extension(&self, files: &mut Vec<String>, extension: &str);

    /// Hashes the archive with the given hasher. File content doesn't need to
    /// be hashed.
    fn hash(&self, hasher: &mut crc32fast::Hasher);
}

pub enum ArchiveType {
    Folder,
    Native,
}

/// A common trait to all writable archives.
pub trait Writable {
    fn add_file(&mut self, path: &str, asset: Vec<u8>);

    fn save(&self) {}
}
