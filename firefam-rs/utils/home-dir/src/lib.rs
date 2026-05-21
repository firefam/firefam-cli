use dirs::home_dir;
use firefam_utils_absolute_path::AbsolutePathBuf;
use std::path::PathBuf;

/// Returns the path to the Firefam configuration directory, which can be
/// specified by the `AGENTS_HOME` environment variable. If not set, this
/// defaults to `~/.agents`.
///
/// - If `AGENTS_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If the environment variable is not set, this function does not verify
///   that the returned directory exists.
pub fn find_firefam_home() -> std::io::Result<AbsolutePathBuf> {
    let agents_home_env = std::env::var("AGENTS_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    let home = home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;

    find_firefam_home_from_env(agents_home_env.as_deref(), home)
}

fn find_firefam_home_from_env(
    agents_home_env: Option<&str>,
    home: PathBuf,
) -> std::io::Result<AbsolutePathBuf> {
    if let Some(val) = agents_home_env {
        return resolve_home_env("AGENTS_HOME", val);
    }

    AbsolutePathBuf::from_absolute_path(home.join(".agents"))
}

fn resolve_home_env(env_var: &str, val: &str) -> std::io::Result<AbsolutePathBuf> {
    let path = PathBuf::from(val);
    let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{env_var} points to {val:?}, but that path does not exist"),
        ),
        _ => std::io::Error::new(
            err.kind(),
            format!("failed to read {env_var} {val:?}: {err}"),
        ),
    })?;

    if !metadata.is_dir() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{env_var} points to {val:?}, but that path is not a directory"),
        ))
    } else {
        let canonical = path.canonicalize().map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("failed to canonicalize {env_var} {val:?}: {err}"),
            )
        })?;
        AbsolutePathBuf::from_absolute_path(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::find_firefam_home_from_env;
    use firefam_utils_absolute_path::AbsolutePathBuf;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::io::ErrorKind;
    use tempfile::TempDir;

    #[test]
    fn find_firefam_home_env_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-agents-home");
        let missing_str = missing
            .to_str()
            .expect("missing agents home path should be valid utf-8");

        let err = find_firefam_home_from_env(Some(missing_str), temp_home.path().to_path_buf())
            .expect_err("missing AGENTS_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("AGENTS_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_firefam_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("agents-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file agents home path should be valid utf-8");

        let err = find_firefam_home_from_env(Some(file_str), temp_home.path().to_path_buf())
            .expect_err("file AGENTS_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_firefam_home_env_valid_directory_canonicalizes() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp agents home path should be valid utf-8");

        let resolved = find_firefam_home_from_env(Some(temp_str), temp_home.path().to_path_buf())
            .expect("valid AGENTS_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_firefam_home_without_env_uses_default_home_dir() {
        let temp_home = TempDir::new().expect("temp home");
        let resolved = find_firefam_home_from_env(
            /*agents_home_env*/ None,
            temp_home.path().to_path_buf(),
        )
        .expect("default AGENTS_HOME");
        let expected = temp_home.path().join(".agents");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }
}
