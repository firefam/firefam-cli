use anyhow::Result;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

fn firefam_command(firefam_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(firefam_utils_cargo_bin::cargo_bin("firefam")?);
    cmd.env("FIREFAM_HOME", firefam_home);
    Ok(cmd)
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn update_does_not_start_interactive_prompt() -> Result<()> {
    let firefam_home = TempDir::new()?;

    firefam_command(firefam_home.path())?
        .arg("update")
        .assert()
        .failure()
        .stderr(contains(
            "`firefam update` is not available in debug builds",
        ));

    Ok(())
}
