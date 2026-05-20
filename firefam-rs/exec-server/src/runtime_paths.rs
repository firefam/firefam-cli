use std::path::PathBuf;

use firefam_utils_absolute_path::AbsolutePathBuf;

/// Runtime paths needed by exec-server child processes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecServerRuntimePaths {
    /// Stable path to the Firefam executable used to launch hidden helper modes.
    pub firefam_self_exe: AbsolutePathBuf,
    /// Path to the Linux sandbox helper alias used when the platform sandbox
    /// needs to re-enter Firefam by argv0.
    pub firefam_linux_sandbox_exe: Option<AbsolutePathBuf>,
}

impl ExecServerRuntimePaths {
    pub fn from_optional_paths(
        firefam_self_exe: Option<PathBuf>,
        firefam_linux_sandbox_exe: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        let firefam_self_exe = firefam_self_exe.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Firefam executable path is not configured",
            )
        })?;
        Self::new(firefam_self_exe, firefam_linux_sandbox_exe)
    }

    pub fn new(
        firefam_self_exe: PathBuf,
        firefam_linux_sandbox_exe: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            firefam_self_exe: absolute_path(firefam_self_exe)?,
            firefam_linux_sandbox_exe: firefam_linux_sandbox_exe.map(absolute_path).transpose()?,
        })
    }
}

fn absolute_path(path: PathBuf) -> std::io::Result<AbsolutePathBuf> {
    AbsolutePathBuf::from_absolute_path(path.as_path())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
}
