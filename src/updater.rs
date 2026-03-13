use anyhow::Result;
use self_update::cargo_crate_version;

/// Checks for updates and returns the new version string if an update is available.
pub async fn check_for_updates() -> Result<Option<String>> {
    tokio::task::spawn_blocking(|| {
        let status = self_update::backends::github::Update::configure()
            .repo_owner("Rutra09")
            .repo_name("librus_desktop")
            .bin_name("librus-front")
            .show_download_progress(false)
            .current_version(cargo_crate_version!())
            .build()?
            .get_latest_release()?;
            
        let current_version = cargo_crate_version!();
        if self_update::version::bump_is_greater(current_version, &status.version)? {
            Ok(Some(status.version))
        } else {
            Ok(None)
        }
    }).await?
}

/// Downloads and installs the update.
pub async fn install_update() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        self_update::backends::github::Update::configure()
            .repo_owner("Rutra09")
            .repo_name("librus_desktop")
            .bin_name("librus-front")
            .show_download_progress(false)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;
        Ok(())
    }).await?
}
