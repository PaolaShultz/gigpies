//! Durable device identity reservation shared by explicitly activated source hosts.
use std::{
    fs::{File, OpenOptions},
    os::fd::AsRawFd,
    path::PathBuf,
};
type Result<T> = std::result::Result<T, String>;
/// Held for the process lifetime, preventing competing owners of this durable
/// epoch ledger. Every configuration replacement durably reserves a fresh epoch.
pub(crate) struct Epoch {
    _lock: File,
    path: PathBuf,
    pub(crate) value: u64,
}
impl Epoch {
    pub(crate) fn open(path: &std::path::Path) -> Result<Self> {
        Self::open_at_least(path, 1)
    }
    pub(crate) fn open_at_least(path: &std::path::Path, minimum: u64) -> Result<Self> {
        if minimum == 0 {
            return Err("nonzero device epoch minimum required".into());
        }
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let parent = path.parent().ok_or("epoch parent")?;
        let metadata = std::fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
        if !path.is_absolute()
            || !metadata.is_dir()
            || metadata.permissions().mode() & 0o777 != 0o700
        {
            return Err("private0700 epoch parent required".into());
        }
        let lock_path = path.with_extension("lock");
        if lock_path == path {
            return Err("epoch ledger must differ from its lock path".into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        // SAFETY: lock owns a valid descriptor; advisory lock remains on a stable
        // separate inode while the high-water file is atomically replaced.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("device epoch owner already active".into());
        }
        let value = match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                if !meta.is_file() || meta.len() == 0 || meta.len() > 32 {
                    return Err("epoch ledger invalid; refusing reuse".into());
                }
                serde_json::from_slice::<u64>(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|_| "epoch ledger invalid; refusing reuse")?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e.to_string()),
        };
        let mut result = Self {
            _lock: lock,
            path: path.into(),
            value: value.max(minimum - 1),
        };
        result.advance()?;
        Ok(result)
    }
    pub(crate) fn advance(&mut self) -> Result<u64> {
        let next = self.value.checked_add(1).ok_or("device epoch exhausted")?;
        crate::show::persist(
            self.path.parent().ok_or("epoch parent")?,
            self.path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("epoch filename")?,
            &next,
        )?;
        self.value = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn requested_minimum_never_reuses_a_reserved_source_epoch() {
        let dir = std::env::temp_dir().join(format!("gp-source-epoch-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("epoch.json");
        let collision = dir.join("epoch.lock");
        assert!(Epoch::open(&collision).is_err());
        assert!(!collision.exists());
        assert!(Epoch::open_at_least(&path, 0).is_err());
        let first = Epoch::open_at_least(&path, 700).unwrap();
        assert_eq!(first.value, 700);
        assert!(Epoch::open_at_least(&path, 900).is_err());
        drop(first);
        assert_eq!(Epoch::open_at_least(&path, 700).unwrap().value, 701);
        assert_eq!(Epoch::open_at_least(&path, 900).unwrap().value, 900);
        assert_eq!(Epoch::open(&path).unwrap().value, 901);
        std::fs::write(&path, u64::MAX.to_string()).unwrap();
        assert!(Epoch::open(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
