use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn firefam_command(firefam_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(firefam_utils_cargo_bin::cargo_bin("firefam")?);
    cmd.env("AGENTS_HOME", firefam_home);
    Ok(cmd)
}

#[test]
fn strict_config_rejects_unknown_config_override() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["--strict-config", "-c", "foo=bar", "mcp-server"])
        .assert()
        .failure()
        .stderr(contains("unknown configuration field"));

    Ok(())
}

#[test]
fn cloud_command_is_not_registered() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["cloud", "list"])
        .assert()
        .failure()
        .stderr(contains("unexpected argument 'list' found"));

    Ok(())
}

#[tokio::test]
async fn features_enable_writes_feature_flag_to_config() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["features", "enable", "unified_exec"])
        .assert()
        .success()
        .stdout(contains("Enabled feature `unified_exec` in config.toml."));

    let config = std::fs::read_to_string(firefam_home.path().join("firefam-config.toml"))?;
    assert!(config.contains("[features]"));
    assert!(config.contains("unified_exec = true"));

    Ok(())
}

#[tokio::test]
async fn features_disable_writes_feature_flag_to_config() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["features", "disable", "shell_tool"])
        .assert()
        .success()
        .stdout(contains("Disabled feature `shell_tool` in config.toml."));

    let config = std::fs::read_to_string(firefam_home.path().join("firefam-config.toml"))?;
    assert!(config.contains("[features]"));
    assert!(config.contains("shell_tool = false"));

    Ok(())
}

#[tokio::test]
async fn features_enable_under_development_feature_prints_warning() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["features", "enable", "runtime_metrics"])
        .assert()
        .success()
        .stderr(contains(
            "Under-development features enabled: runtime_metrics.",
        ));

    Ok(())
}

#[tokio::test]
async fn features_list_is_sorted_alphabetically_by_feature_name() -> Result<()> {
    let firefam_home = TempDir::new()?;

    let mut cmd = firefam_command(firefam_home.path())?;
    let output = cmd
        .args(["features", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    let actual_names = stdout
        .lines()
        .map(|line| {
            line.split_once("  ")
                .map(|(name, _)| name.trim_end().to_string())
                .expect("feature list output should contain aligned columns")
        })
        .collect::<Vec<_>>();
    let mut expected_names = actual_names.clone();
    expected_names.sort();

    assert_eq!(actual_names, expected_names);

    Ok(())
}
