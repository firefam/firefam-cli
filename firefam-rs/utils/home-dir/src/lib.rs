use dirs::home_dir;
use firefam_utils_absolute_path::AbsolutePathBuf;
use std::path::PathBuf;

/// Returns the path to the Firefam configuration directory, which can be
/// specified by the `FIREFAM_HOME` environment variable. If not set, the legacy
/// `CODEX_HOME` variable is honored for authentication/config compatibility;
/// otherwise this defaults to `~/.firefam`, or `~/.codex` when that legacy
/// directory already exists and `~/.firefam` does not.
///
/// - If `FIREFAM_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `CODEX_HOME` is used, the same validation rules apply.
/// - If neither environment variable is set, this function does not verify that
///   the returned directory exists.
pub fn find_firefam_home() -> std::io::Result<AbsolutePathBuf> {
    let firefam_home_env = std::env::var("FIREFAM_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    let codex_home_env = std::env::var("CODEX_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    let home = home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;

    find_firefam_home_from_env(firefam_home_env.as_deref(), codex_home_env.as_deref(), home)
}

fn find_firefam_home_from_env(
    firefam_home_env: Option<&str>,
    codex_home_env: Option<&str>,
    home: PathBuf,
) -> std::io::Result<AbsolutePathBuf> {
    // Honor the `FIREFAM_HOME` environment variable when it is set to allow users
    // (and tests) to override the default location.
    if let Some(val) = firefam_home_env {
        return resolve_home_env("FIREFAM_HOME", val);
    }

    if let Some(val) = codex_home_env {
        return resolve_home_env("CODEX_HOME", val);
    }

    let firefam_home = home.join(".firefam");
    let codex_home = home.join(".codex");
    let path = if !firefam_home.exists() && codex_home.is_dir() {
        codex_home
    } else {
        firefam_home
    };
    AbsolutePathBuf::from_absolute_path(path)
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
        let missing = temp_home.path().join("missing-firefam-home");
        let missing_str = missing
            .to_str()
            .expect("missing firefam home path should be valid utf-8");

        let err =
            find_firefam_home_from_env(Some(missing_str), None, temp_home.path().to_path_buf())
                .expect_err("missing FIREFAM_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("FIREFAM_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_firefam_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("firefam-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file firefam home path should be valid utf-8");

        let err = find_firefam_home_from_env(Some(file_str), None, temp_home.path().to_path_buf())
            .expect_err("file FIREFAM_HOME");
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
            .expect("temp firefam home path should be valid utf-8");

        let resolved =
            find_firefam_home_from_env(Some(temp_str), None, temp_home.path().to_path_buf())
                .expect("valid FIREFAM_HOME");
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
            /*firefam_home_env*/ None,
            None,
            temp_home.path().to_path_buf(),
        )
        .expect("default FIREFAM_HOME");
        let expected = temp_home.path().join(".firefam");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_firefam_home_uses_legacy_codex_home_env_when_firefam_home_is_unset() {
        let temp_home = TempDir::new().expect("temp home");
        let codex_home = temp_home.path().join("codex-home");
        fs::create_dir(&codex_home).expect("create legacy home");
        let codex_home_str = codex_home
            .to_str()
            .expect("legacy codex home path should be valid utf-8");

        let resolved =
            find_firefam_home_from_env(None, Some(codex_home_str), temp_home.path().to_path_buf())
                .expect("legacy CODEX_HOME");
        let expected = codex_home.canonicalize().expect("canonicalize legacy home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_firefam_home_without_env_prefers_existing_legacy_codex_dir() {
        let temp_home = TempDir::new().expect("temp home");
        let codex_home = temp_home.path().join(".codex");
        fs::create_dir(&codex_home).expect("create legacy home");

        let resolved = find_firefam_home_from_env(
            /*firefam_home_env*/ None,
            None,
            temp_home.path().to_path_buf(),
        )
        .expect("legacy default home");
        let expected = codex_home;
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }
}
