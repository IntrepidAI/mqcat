use std::{io::Read, path::Path};

use anyhow::{bail, Context};
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(bin_name = "mqcat")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(styles = crate::cli::get_styles())]
#[command(override_usage = "mqcat [OPTIONS] --upgrade [VERSION]")]
pub struct UpgradeArgs {
    #[arg(global = true, short, long, action = clap::ArgAction::Count, conflicts_with = "quiet")]
    /// increase logging verbosity
    verbose: u8,
    #[arg(global = true, short, long, action = clap::ArgAction::Count, conflicts_with = "verbose")]
    /// decrease logging verbosity
    quiet: u8,
    #[arg(global = true, short, long, action = clap::ArgAction::Help)]
    /// print this help message
    help: Option<bool>,
    #[arg(long, hide = true)]
    /// upgrade executable to the latest version
    upgrade: bool,
    /// upgrade to a specific version (v0.0.0) or release channel (stable, dev)
    version: Option<String>,
    #[arg(short, long)]
    /// upgrade even if already up-to-date
    force: bool,
    #[arg(short, long)]
    /// dry run, do not actually upgrade
    dry_run: bool,
    #[arg(short, long)]
    /// fetch binary for a specific cpu architecture (e.g. x86_64-linux)
    arch: Option<String>,
}

pub async fn run_app(args: Vec<String>) {
    let args = UpgradeArgs::parse_from(args);
    crate::cli::setup_logging(args.verbose, args.quiet);
    crate::cli::ctrlc_trap(async move {
        upgrade(args.version, args.force, args.dry_run, args.arch).await
    }).await;
}

// these are characters that can appear in semver versions or architecture triples
fn check_valid_chars(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '+' | '_'))
}

pub async fn upgrade(version: Option<String>, force: bool, dry_run: bool, arch: Option<String>) -> anyhow::Result<()> {
    let version = version.unwrap_or_else(|| "latest".to_owned());
    let arch = arch.unwrap_or_else(|| crate::version::get_target_triple(false));

    if !check_valid_chars(&version) {
        bail!("invalid version: {}", version);
    }

    if !check_valid_chars(&arch) {
        bail!("invalid arch: {}", arch);
    }

    #[derive(Debug, Deserialize)]
    struct GithubRelease {
        tag_name: String,
        updated_at: String,
        assets: Vec<GithubAsset>,
    }

    #[derive(Debug, Deserialize)]
    struct GithubAsset {
        name: String,
        browser_download_url: String,
    }

    // let release_channel = format!("{}-{}", version, arch);
    let manifest_url = format!("https://api.github.com/repos/IntrepidAI/mqcat/releases/{}", version);
    log::info!("downloading manifest from {}", manifest_url);

    let manifest_response = ureq::get(manifest_url)
        .call()?
        .body_mut()
        .read_json::<GithubRelease>()?;

    if manifest_response.tag_name == crate::version::get_version() && !force {
        log::info!("already up to date");
        return Ok(());
    }

    let updated_date = manifest_response.updated_at.split('T').next().unwrap_or_default();
    if updated_date < crate::version::get_build_date() && !force {
        log::warn!("your build is newer than the latest release");
        log::warn!("current build date: {}", crate::version::get_build_date());
        log::warn!("release build date: {}", updated_date);
        log::warn!("use --force if you want to downgrade");
        return Ok(());
    }

    log::info!("upgrading to {}", manifest_response.tag_name);

    let update_file_name = format!("mqcat-{}-{}.zip", manifest_response.tag_name.trim_start_matches('v'), arch);
    let mut update_file_url = None;

    for asset in manifest_response.assets {
        if asset.name == update_file_name {
            update_file_url = Some(asset.browser_download_url);
            break;
        }
    }

    let update_file_url = update_file_url.with_context(|| format!("file not found: {}", update_file_name))?;
    log::info!("downloading file from {}", update_file_url);

    let bytes_request = ureq::get(update_file_url)
        .call()?
        .body_mut()
        .read_to_vec()?;

    log::info!("downloaded {} bytes", bytes_request.len());

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes_request))?;
    let arch_windows = arch.ends_with("-windows");
    let mut zipfile = archive.by_name(if arch_windows { "mqcat.exe" } else { "mqcat" })?;
    let mut binary = vec![];
    zipfile.read_to_end(&mut binary)?;

    if dry_run {
        log::info!("upgrade done (dry run)");
        return Ok(());
    }

    let current_exe = std::env::current_exe()?;
    let tmp_exe = if cfg!(windows) {
        current_exe.with_extension("new.exe")
    } else {
        current_exe.with_extension("new")
    };

    std::fs::write(&tmp_exe, &binary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_exe)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_exe, perms)?;
    }

    replace_exe(&tmp_exe, &current_exe)?;

    log::info!("upgrade done");
    Ok(())
}

fn replace_exe(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    if cfg!(windows) {
        std::fs::rename(to, to.with_extension("old.exe"))?;
    } else {
        std::fs::remove_file(to)?;
    }
    // rename may fail across device boundaries
    std::fs::rename(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))?;
    Ok(())
}
