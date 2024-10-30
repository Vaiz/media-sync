use bitflags::bitflags;
use std::time::{SystemTime, UNIX_EPOCH};

bitflags! {
    #[derive(Debug, Clone)]
    struct FileFlags: u8 {
        const NONE = 0b000;
        const IS_DIR = 0b001;
        const IS_FILE = 0b010;
        const IS_SYMLINK = 0b100;
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    modified: SystemTime,
    flags: FileFlags,
}

impl Metadata {
    pub fn dummy_folder() -> Self {
        Self {
            len: 0,
            modified: SystemTime::now(),
            flags: FileFlags::IS_DIR,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.flags.contains(FileFlags::IS_DIR)
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn modified(&self) -> SystemTime {
        self.modified
    }
}

impl From<std::fs::Metadata> for Metadata {
    fn from(metadata: std::fs::Metadata) -> Self {
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        let mut flags = FileFlags::NONE;

        if metadata.is_dir() {
            flags.insert(FileFlags::IS_DIR);
        }
        if metadata.is_file() {
            flags.insert(FileFlags::IS_FILE);
        }
        if metadata.is_symlink() {
            flags.insert(FileFlags::IS_SYMLINK);
        }

        Self {
            len: metadata.len(),
            modified,
            flags,
        }
    }
}
