use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use tempfile::TempDir;

fn firefam_command(firefam_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(firefam_utils_cargo_bin::cargo_bin("firefam")?);
    cmd.env("AGENTS_HOME", firefam_home);
    Ok(cmd)
}

#[test]
fn strict_config_rejects_unknown_config_fields_for_app_server() -> Result<()> {
    let firefam_home = TempDir::new()?;
    std::fs::write(
        firefam_home.path().join("firefam-config.toml"),
        r#"
foo = "bar"
"#,
    )?;

    let mut cmd = firefam_command(firefam_home.path())?;
    cmd.args(["app-server", "--strict-config", "--listen", "off"])
        .assert()
        .failure()
        .stderr(contains("unknown configuration field"));

    Ok(())
}
