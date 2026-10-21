//! Private control channel for the external debug build/restart supervisor.
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Clone)]
pub(crate) struct HotReload {
    directory: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Status {
    pub available: bool,
    pub enabled: bool,
    pub detail: String,
}

impl HotReload {
    pub fn from_environment() -> Self {
        Self {
            directory: std::env::var_os("GPUI_AGENTATION_HOT_RELOAD").map(PathBuf::from),
        }
    }

    #[cfg(test)]
    pub fn for_directory(directory: PathBuf) -> Self {
        Self {
            directory: Some(directory),
        }
    }

    pub fn status(&self) -> Status {
        let read = || -> io::Result<Status> {
            let directory = self
                .directory
                .as_ref()
                .ok_or_else(|| io::Error::other("No supervisor"))?;
            let enabled = std::fs::read_to_string(directory.join("enabled"))?;
            let detail = std::fs::read_to_string(directory.join("status"))?;
            Ok(Status {
                available: true,
                enabled: enabled.trim() == "1",
                detail,
            })
        };
        read().unwrap_or_else(|_| Status {
            available: false,
            enabled: false,
            detail: "Start with the Agentation dev runner to enable hot reload.".into(),
        })
    }

    pub fn set_enabled(&self, enabled: bool) -> io::Result<Status> {
        let directory = self
            .directory
            .as_ref()
            .ok_or_else(|| io::Error::other("No hot reload supervisor"))?;
        // Do not recreate a stale supervisor directory after the runner exits.
        if !directory.join("enabled").is_file() {
            return Err(io::Error::other("The hot reload supervisor has stopped"));
        }
        let mut file = atomic_write_file::AtomicWriteFile::open(directory.join("enabled"))?;
        file.write_all(if enabled { b"1" } else { b"0" })?;
        file.commit()?;
        let mut status = self.status();
        status.detail = if enabled { "Starting" } else { "Off" }.into();
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_controls_the_supervisor_without_creating_stale_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("enabled"), "0").unwrap();
        std::fs::write(directory.path().join("status"), "Off").unwrap();
        let control = HotReload {
            directory: Some(directory.path().into()),
        };
        assert!(!control.status().enabled);
        assert!(control.set_enabled(true).unwrap().enabled);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("enabled")).unwrap(),
            "1"
        );
        assert!(!control.set_enabled(false).unwrap().enabled);
        directory.close().unwrap();
        assert!(!control.status().available);
        assert!(control.set_enabled(true).is_err());
    }

    #[test]
    fn ordinary_launch_explains_why_hot_reload_is_unavailable() {
        let status = HotReload { directory: None }.status();
        assert!(!status.available);
        assert!(!status.enabled);
        assert!(status.detail.contains("dev runner"));
    }
}
