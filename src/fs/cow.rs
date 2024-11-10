use crate::fs::{Fs, Metadata};
use anyhow::Context;
use reflink_copy::ReflinkSupport;
use std::path::Path;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU64, AtomicU8};

#[derive(Debug, Copy, Clone)]
enum ReflinkState {
    ForceReflink = 0,
    ReflinkOrCopy = 1,
    Copy = 2,
}

const MAX_FAILS_COUNT: u64 = 10;

pub(crate) struct CowFs<T> {
    fs: T,
    reflink_state: AtomicU8,
    success_reflinks: AtomicU64,
    failed_reflinks: AtomicU64,
}

impl<T> CowFs<T> {
    pub(crate) fn new(fs: T, support: ReflinkSupport) -> Self {
        assert_ne!(
            support,
            ReflinkSupport::NotSupported,
            "cannot create CowFs for Unsupported fs"
        );

        let reflink_state = match support {
            ReflinkSupport::Supported => ReflinkState::ForceReflink,
            ReflinkSupport::NotSupported => {
                panic!("cannot create CowFs for Unsupported fs")
            }
            ReflinkSupport::Unknown => ReflinkState::ReflinkOrCopy,
        };

        Self {
            fs,
            reflink_state: AtomicU8::new(reflink_state as u8),
            success_reflinks: AtomicU64::new(0),
            failed_reflinks: AtomicU64::new(0),
        }
    }
}

impl<T: Fs> Fs for CowFs<T> {
    fn name(&self) -> String {
        format!("CoW({})", self.fs.name())
    }
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        self.fs.create_dir_all(path)
    }

    fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        self.fs.metadata(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<u64> {
        match self.reflink_state.load(Relaxed) {
            0 => reflink_copy::reflink(&from, &to)
                .with_context(|| {
                    format!("failed to reflink {} to {}", from.display(), to.display())
                })
                .map(|_| 0),
            1 => match reflink_copy::reflink_or_copy(&from, &to)? {
                None => {
                    self.success_reflinks.fetch_add(1, Relaxed);
                    Ok(0)
                }
                Some(size) => {
                    let fails_count = self.failed_reflinks.fetch_add(1, Relaxed);
                    if fails_count > MAX_FAILS_COUNT && self.success_reflinks.load(Relaxed) == 0 {
                        self.reflink_state.store(ReflinkState::Copy as u8, Relaxed);
                        eprintln!("reflink doesn't work, permanently switching to copy");
                    }
                    Ok(size)
                }
            },
            _ => self.fs.copy(from, to),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        self.fs.exists(path)
    }
}
