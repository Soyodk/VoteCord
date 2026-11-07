//! Update checks: the newest release of Votecord on GitHub, against this
//! build's version. One is offered in a bar over the window, whose Update
//! button fetches the installer from the release and starts it; where the
//! release has no installer of this build's shape, the button opens the
//! releases page instead.
//!
//! The check asks GitHub for the newest release, whatever its prerelease
//! state, as the nightly goes out as one: a build made here with a release
//! version, such as a source build at 0.3.0, never counts one of its own
//! prereleases as newer. Nothing is sent but the request itself; Settings →
//! Advanced turns the checks off.

use serde::Deserialize;

use super::*;

/// The repository's releases, newest first, of any prerelease state.
const RELEASES: &str = "https://api.github.com/repos/Soyodk/VoteCord/releases?per_page=1";
/// The releases page, for the platforms whose update is a download of theirs
/// rather than this program's to run.
const RELEASES_PAGE: &str = "https://github.com/Soyodk/VoteCord/releases";
/// The first check waits this long after the window shows, so it never makes
/// starting slower; the ones after it space this far apart.
const FIRST_CHECK: std::time::Duration = std::time::Duration::from_secs(10);
const CHECK_AGAIN: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);
/// The request names itself, as GitHub's API asks.
const USER_AGENT: &str = concat!("Votecord/", env!("CARGO_PKG_VERSION"));

/// A release and what it offers, as the API answers.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Starts the checks: one shortly after the window shows, then one every
/// [`CHECK_AGAIN`], each on the background runtime, whose answer lands on the
/// UI thread. A turn the Settings switch has off checks nothing, and keeps
/// the cycle, so turning it on checks at the next one.
pub(super) fn start(adapter: &Rc<RefCell<Adapter>>) {
    schedule(adapter, FIRST_CHECK);
}

/// Sets the timer for the next check.
fn schedule(adapter: &Rc<RefCell<Adapter>>, at: std::time::Duration) {
    // A weak holds the cycle, which the timer itself lives in: a strong one
    // would keep the adapter forever.
    let state = Rc::downgrade(adapter);
    adapter
        .borrow_mut()
        .update_timer
        .start(slint::TimerMode::SingleShot, at, move || {
            let Some(adapter) = state.upgrade() else {
                return;
            };
            // Off stays off for this turn, without stopping the cycle.
            let (wanted, window, runtime) = {
                let borrowed = adapter.borrow();
                (
                    borrowed
                        .window
                        .upgrade()
                        .is_some_and(|window| window.get_check_updates()),
                    borrowed.window.clone(),
                    borrowed.runtime.clone(),
                )
            };
            if wanted {
                check(runtime, window);
            }
            schedule(&adapter, CHECK_AGAIN);
        });
}

/// Asks for the newest release and, when it is newer than this build and not
/// the one dismissed, shows it on the bar.
fn check(runtime: Arc<BackgroundRuntime>, window: slint::Weak<VotecordWindow>) {
    runtime.spawn(async move {
        let found = latest_release().await;
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            let release = match found {
                Ok(release) => release,
                Err(error) => {
                    tracing::debug!(%error, "the update check failed");
                    return;
                }
            };
            let version = release.tag_name.trim_start_matches('v').to_owned();
            let newer = update_newer(&version, env!("CARGO_PKG_VERSION"));
            let dismissed = window.get_update_dismissed_version();
            if newer && dismissed != version {
                let url = installer(&release)
                    .map_or_else(String::new, |asset| asset.browser_download_url.clone());
                window.set_update_asset(url.into());
                window.set_update_version(version.into());
            } else if !newer {
                window.set_update_asset("".into());
            }
        });
    });
}

/// The newest release, of any prerelease state, as GitHub answers.
async fn latest_release() -> anyhow::Result<Release> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("could not set up HTTPS")?;
    let release: Release = serde_json::from_str(
        &http
            .get(RELEASES)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("could not reach GitHub")?
            .error_for_status()
            .context("GitHub refused the update check")?
            .text()
            .await
            .context("GitHub answered something unreadable")?,
    )
    .context("GitHub answered something else than a release")?;
    Ok(release)
}

/// Whether `latest` is a newer version than this build's, by SemVer's order:
/// the numbers first, then a release over any prerelease, then prerelease
/// parts each, numbers by size, a number under a word, and a longer list over
/// a shorter one.
fn update_newer(latest: &str, current: &str) -> bool {
    let numbers = |version: &str| -> Option<(u64, u64, u64)> {
        // The core numbers come first, whatever prerelease follows them.
        let core = version.split(['-', '+']).next()?;
        let mut parts = core.split('.');
        let number = |part: Option<&str>| part?.parse::<u64>().ok();
        Some((
            number(parts.next())?,
            number(parts.next())?,
            number(parts.next())?,
        ))
    };
    let (Some(latest_numbers), Some(current_numbers)) = (numbers(latest), numbers(current)) else {
        return false;
    };
    if latest_numbers != current_numbers {
        return latest_numbers > current_numbers;
    }
    // What follows the numbers: a release has nothing there and a prerelease
    // is always under one, so only two prereleases go on to their parts.
    fn prerelease(version: &str) -> Option<&str> {
        version
            .split_once('-')
            .map(|(_, pre)| pre.split('+').next().unwrap_or(pre))
    }
    let (Some(latest_pre), Some(current_pre)) = (prerelease(latest), prerelease(current)) else {
        return prerelease(latest).is_none() && prerelease(current).is_some();
    };
    let parts = |version: &str| version.split('.').map(String::from).collect::<Vec<_>>();
    let (latest, current) = (parts(latest_pre), parts(current_pre));
    for index in 0..latest.len().max(current.len()) {
        match (latest.get(index), current.get(index)) {
            (None, None) => break,
            (None, Some(_)) => return false, // the shorter side is behind.
            (Some(_), None) => return true,
            (Some(a), Some(b)) => {
                let (number, word) = (a.parse::<u64>().ok(), b.parse::<u64>().ok());
                match (number, word) {
                    (Some(a), Some(b)) if a != b => return a > b,
                    (Some(_), Some(_)) => {}
                    (Some(_), None) => return false, // a number under a word.
                    (None, Some(_)) => return true,
                    (None, None) if a != b => return a > b,
                    (None, None) => {}
                }
            }
        }
    }
    false
}

/// The asset to run for an update, by this build's shape: the Windows
/// installer, the full preview's on a Matrix build, and none elsewhere, where
/// the releases page is what the button opens.
fn installer(release: &Release) -> Option<&Asset> {
    if !cfg!(windows) {
        return None;
    }
    let full = cfg!(feature = "matrix");
    release.assets.iter().find(|asset| {
        asset.name.contains("windows-x64-setup")
            && asset.name.ends_with(".exe")
            && asset.name.contains("full-preview") == full
    })
}

/// The Update button: downloads the installer and starts it, then Votecord
/// quits so it can be replaced; without an installer of this build's shape,
/// the releases page opens instead.
pub(super) fn start_download(adapter: &Rc<RefCell<Adapter>>) {
    let (runtime, window, url) = {
        let borrowed = adapter.borrow();
        let Some(window) = borrowed.window.upgrade() else {
            return;
        };
        // The address is the one the check left on the window; without one,
        // this platform's update is a download the page holds.
        let url = window.get_update_asset().to_string();
        if url.is_empty() {
            drop(borrowed);
            crate::ui::browser::open(RELEASES_PAGE);
            return;
        }
        window.set_update_downloading(true);
        window.set_update_note(SharedString::default());
        (borrowed.runtime.clone(), window.as_weak(), url)
    };
    runtime.spawn(async move {
        let downloaded = download(&url).await;
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            window.set_update_downloading(false);
            match downloaded {
                Ok(_) => {
                    // The installer replaces this program, which mustn't be
                    // running when it does.
                    let _ = slint::quit_event_loop();
                }
                Err(error) => {
                    tracing::warn!(%error, "the update download failed");
                    window.set_update_note(format!("The download failed: {error}").into());
                }
            }
        });
    });
}

/// Streams the installer into the temp folder, whole, for the caller to start.
async fn download(url: &str) -> anyhow::Result<std::path::PathBuf> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .context("could not set up HTTPS")?;
    let mut response = http
        .get(url)
        .send()
        .await
        .context("could not reach the download")?
        .error_for_status()
        .context("the download was refused")?;
    let name = url.rsplit('/').next().unwrap_or("votecord-setup.exe");
    let path = std::env::temp_dir().join(name);
    let mut file = std::fs::File::create(&path).context("could not write the installer")?;
    while let Some(chunk) = response.chunk().await.context("the download broke")? {
        use std::io::Write;
        file.write_all(&chunk)
            .context("could not write the installer")?;
    }
    file.sync_all().ok();
    drop(file);
    // A whole installer starts itself over this one, which is quitting as it
    // does; its own prompt sees to a window still up.
    let child = std::process::Command::new(&path)
        .spawn()
        .context("could not start the installer")?;
    tracing::info!(?path, child = child.id(), "the update installer started");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::update_newer;

    #[test]
    fn orders_core_versions_and_stable_releases() {
        assert!(update_newer("1.10.0", "1.9.9"));
        assert!(update_newer("1.0.0", "1.0.0-rc.1"));
        assert!(!update_newer("1.0.0-rc.1", "1.0.0"));
        assert!(!update_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn orders_prerelease_identifiers_by_semver_rules() {
        assert!(update_newer("1.0.0-alpha.10", "1.0.0-alpha.2"));
        assert!(update_newer("1.0.0-alpha.2", "1.0.0-alpha"));
        assert!(update_newer("1.0.0-alpha", "1.0.0-2"));
        assert!(!update_newer("1.0.0-2", "1.0.0-alpha"));
    }

    #[test]
    fn compares_nightly_versions_and_ignores_build_metadata() {
        assert!(update_newer(
            "0.4.0-nightly.20261107.1",
            "0.4.0-nightly.20261106.9"
        ));
        assert!(!update_newer("1.0.0+build.2", "1.0.0+build.1"));
        assert!(!update_newer("1.0.0-alpha+build.2", "1.0.0-alpha+build.1"));
    }
}
