use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Error};
use wasmer_pack_testing::Language;

fn main() -> Result<(), Error> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    autodiscover(&crate_root, Language::Python)?;
    Ok(())
}

fn autodiscover(crate_dir: impl AsRef<Path>, language: Language) -> Result<(), Error> {
    let crate_dir = crate_dir.as_ref();
    tracing::info!(dir = %crate_dir.display(), "Looking for tests");

    let manifest_path = crate_dir.join("Cargo.toml");
    let temp = tempfile::tempdir().context("Unable to create a temporary directory")?;

    tracing::debug!("Compiling the crate and generating a WAPM package");
    let wapm_package =
        wasmer_pack_testing::compile_rust_to_wapm_package(&manifest_path, temp.path())?;

    let generated_bindings = crate_dir.join("generated_bindings");

    if generated_bindings.exists() {
        tracing::debug!("Deleting bindings from a previous run");
        std::fs::remove_dir_all(&generated_bindings)
            .context("Unable to delete the old generated bindings")?;
    }

    tracing::debug!(
        bindings_dir = %generated_bindings.display(),
        "Generating bindings",
    );
    wasmer_pack_testing::generate_bindings(&generated_bindings, &wapm_package, language)?;

    match language {
        Language::JavaScript => todo!(),
        Language::Python => {
            setup_python(crate_dir, &generated_bindings)?;
            run_pytest(crate_dir)?;
        }
    }

    Ok(())
}

fn setup_python(crate_dir: &Path, generated_bindings: &Path) -> Result<(), Error> {
    let pyproject = crate_dir.join("pyproject.toml");

    if pyproject.exists() {
        // Assume everything has been set up correctly
        return Ok(());
    }

    tracing::info!("Initializing the python package");

    let mut cmd = Command::new("poetry");
    cmd.arg("init").arg("--name=tests").arg("--no-interaction");
    tracing::debug!(?cmd, "Initializing the Python package");
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(crate_dir)
        .status()
        .context("Unable to run poetry. Is it installed?")?;
    anyhow::ensure!(status.success(), "Unable to initialize the Python package");

    let mut cmd = Command::new("poetry");
    cmd.arg("add").arg("--no-interaction").arg("pytest");
    tracing::debug!(?cmd, "Adding pytest as a dependency");
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(crate_dir)
        .status()
        .context("Unable to run poetry. Is it installed?")?;
    anyhow::ensure!(status.success(), "Unable to add pytest as a dependency");

    let mut cmd = Command::new("poetry");
    cmd.arg("add")
        .arg("--no-interaction")
        .arg("--editable")
        .arg(generated_bindings);
    tracing::debug!(?cmd, "Adding the generated bindings as a dependency");
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(crate_dir)
        .status()
        .context("Unable to run poetry. Is it installed?")?;
    anyhow::ensure!(
        status.success(),
        "Unable to add the generated bindings as a dependency"
    );

    Ok(())
}

fn run_pytest(crate_dir: &Path) -> Result<(), Error> {
    let mut cmd = Command::new("poetry");
    cmd.arg("run").arg("pytest").arg("--verbose");
    tracing::debug!(?cmd, "Running pytest");
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(crate_dir)
        .status()
        .context("Unable to run poetry. Is it installed?")?;
    anyhow::ensure!(status.success(), "Testing failed");

    Ok(())
}
