//! The links on this account's profile card: buttons that carry a label and
//! an address, as Discord's connections show theirs, but chosen by the
//! account rather than by an application it authorized.
//!
//! Discord has no field for them: its profile editor takes a name, pronouns,
//! an avatar, a banner, a colour and a bio, and nothing else. Votecord keeps
//! these links on this device, one list per Discord account, and shows them
//! on that account's own profile card, under the bio. They travel with the
//! data directory, like the saved session and the cookies, so a backup keeps
//! them.
//!
//! The file holds one list per account, keyed by its Discord user id, and is
//! written whole when a list changes, as the cookies are: a few hundred
//! bytes, and only when the editor saves.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
};

use serde::{Deserialize, Serialize};

/// How many links one profile may carry: Discord's connections take five,
/// and a card longer than that pushes the rest of the profile out of sight.
pub const LINKS_LIMIT: usize = 5;
/// The longest label and address a link takes, in characters.
pub const LABEL_LIMIT: usize = 32;
pub const URL_LIMIT: usize = 200;

/// A button on the profile card: what it says, and where it goes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileLink {
    pub label: String,
    /// The address as it opens: always with `http://` or `https://`.
    pub url: String,
}

/// Why what was written isn't a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkProblem {
    /// The label is empty, or only spaces.
    NoLabel,
    LabelTooLong,
    /// The address is empty, or only spaces.
    NoUrl,
    UrlTooLong,
    /// It isn't an address, or it isn't one over HTTP or HTTPS.
    NotAnAddress,
    /// The profile already has as many links as it takes.
    Full,
}

impl LinkProblem {
    /// What the editor says under the fields.
    pub fn message(self) -> &'static str {
        match self {
            Self::NoLabel => "Give the link a label, so the card says what it opens.",
            Self::LabelTooLong => "A label can be up to 32 characters.",
            Self::NoUrl => "Write the address the link opens.",
            Self::UrlTooLong => "An address can be up to 200 characters.",
            Self::NotAnAddress => "That isn't an address: write one such as example.com/page.",
            Self::Full => "A profile takes up to 5 links. Remove one to add another.",
        }
    }
}

/// Reads a label and an address as the editor wrote them, or says what's
/// wrong with them. An address without a scheme takes `https://`, as a
/// browser's bar does.
pub fn check(label: &str, url: &str) -> Result<ProfileLink, LinkProblem> {
    let label = label.trim();
    if label.is_empty() {
        return Err(LinkProblem::NoLabel);
    }
    if label.chars().count() > LABEL_LIMIT {
        return Err(LinkProblem::LabelTooLong);
    }
    let url = url.trim();
    if url.is_empty() {
        return Err(LinkProblem::NoUrl);
    }
    if url.chars().count() > URL_LIMIT {
        return Err(LinkProblem::UrlTooLong);
    }
    let url = address(url).ok_or(LinkProblem::NotAnAddress)?;
    Ok(ProfileLink {
        label: label.to_owned(),
        url,
    })
}

/// An address with its scheme, when it's one a browser opens over the web.
fn address(url: &str) -> Option<String> {
    // Discord's own links take no scheme; neither does what people paste.
    let (rest, https) = if let Some(rest) = url.strip_prefix("https://") {
        (rest, true)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (rest, false)
    } else if url.contains("://") {
        return None;
    } else {
        (url, true)
    };
    let rest = rest.to_owned();
    // The host, before the path, the query and the fragment: a dot in it, or
    // localhost, and no space or control character anywhere.
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // A user name before an `@` goes, as a browser drops it; an `@` with
    // nothing before it leaves no host at all.
    let host = match authority.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() => host,
        Some(_) => return None,
        None => authority,
    };
    // A port goes with the host it follows.
    let host = host.split(':').next().unwrap_or_default();
    if host.is_empty()
        || !(host.contains('.') || host == "localhost")
        || host.starts_with('.')
        || host.ends_with('.')
        || rest
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return None;
    }
    Some(format!(
        "{}{rest}",
        if https { "https://" } else { "http://" }
    ))
}

/// The links kept for each account, and the file they're in.
struct Store {
    file: PathBuf,
    accounts: BTreeMap<String, Vec<ProfileLink>>,
}

impl Store {
    /// The links in `file`, or none when it isn't there or isn't readable.
    fn open(file: PathBuf) -> Self {
        let accounts = match std::fs::read(&file) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(accounts) => accounts,
                Err(error) => {
                    tracing::warn!(%error, "could not read the saved profile links");
                    BTreeMap::new()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => {
                tracing::warn!(%error, "could not open the saved profile links");
                BTreeMap::new()
            }
        };
        Self { file, accounts }
    }

    /// Writes the lists beside the file, readable only by this user, then
    /// swaps them in, as the cookies are written.
    fn write(&self) -> std::io::Result<()> {
        let partial = self.file.with_extension("partial");
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut writer = std::io::BufWriter::new(options.open(&partial)?);
        serde_json::to_writer(&mut writer, &self.accounts).map_err(std::io::Error::other)?;
        writer.flush()?;
        drop(writer);
        std::fs::rename(&partial, &self.file)
    }
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

/// Loads the links saved in `file`, and saves them there from now on. Called
/// once, with the data directory, as the cookies are.
pub fn keep_in(file: PathBuf) {
    let store = Store::open(file);
    *STORE.lock().unwrap_or_else(PoisonError::into_inner) = Some(store);
}

/// The links kept for `account`, by its Discord user id: none before any is
/// saved, and none where the store isn't loaded.
pub fn load(account: &str) -> Vec<ProfileLink> {
    let store = STORE.lock().unwrap_or_else(PoisonError::into_inner);
    store
        .as_ref()
        .and_then(|store| store.accounts.get(account))
        .cloned()
        .unwrap_or_default()
}

/// Saves `links` for `account`, replacing what it had. A failure to write
/// leaves the copy in memory, so the card shows them until the app ends.
pub fn save(account: &str, links: Vec<ProfileLink>) {
    let mut store = STORE.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(store) = store.as_mut() else {
        return;
    };
    if links.is_empty() {
        store.accounts.remove(account);
    } else {
        store.accounts.insert(account.to_owned(), links);
    }
    if let Err(error) = store.write() {
        tracing::warn!(%error, "could not save the profile links");
    }
}

/// The links of `account`, when `account` is this one, for a profile card.
/// Other people's cards show what Discord sends, which has none of these.
pub fn for_profile(account: Option<&str>, profile: &str) -> Vec<ProfileLink> {
    match account {
        Some(account) if account == profile => load(account),
        _ => Vec::new(),
    }
}

/// Whether `path` is where the links are kept, for the data directory's
/// listing and for tests.
pub fn file_in(directory: &Path) -> PathBuf {
    directory.join("profile-links.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_an_address_as_a_browser_would() {
        let link = check(" My site ", "example.com/page").unwrap();
        assert_eq!(
            link,
            ProfileLink {
                label: "My site".into(),
                url: "https://example.com/page".into()
            }
        );
        assert_eq!(
            check("Blog", "http://blog.example.org").unwrap().url,
            "http://blog.example.org"
        );
        // Discord's own invite links, and a port.
        assert_eq!(
            check("Server", "discord.gg/abc").unwrap().url,
            "https://discord.gg/abc"
        );
        assert_eq!(
            check("Local", "localhost:3000/me").unwrap().url,
            "https://localhost:3000/me"
        );
        // A scheme that isn't the web's, and something that isn't a host.
        assert_eq!(
            check("Bad", "javascript:alert(1)"),
            Err(LinkProblem::NotAnAddress)
        );
        assert_eq!(
            check("Bad", "ftp://example.com"),
            Err(LinkProblem::NotAnAddress)
        );
        assert_eq!(check("Bad", "no host here"), Err(LinkProblem::NotAnAddress));
        assert_eq!(check("Bad", "example"), Err(LinkProblem::NotAnAddress));
        assert_eq!(check("Bad", "https://"), Err(LinkProblem::NotAnAddress));
        assert_eq!(check("Bad", "@example.com"), Err(LinkProblem::NotAnAddress));
    }

    #[test]
    fn says_what_a_link_lacks() {
        assert_eq!(check("  ", "example.com"), Err(LinkProblem::NoLabel));
        assert_eq!(
            check(&"a".repeat(LABEL_LIMIT + 1), "example.com"),
            Err(LinkProblem::LabelTooLong)
        );
        assert_eq!(check("Site", "  "), Err(LinkProblem::NoUrl));
        let long = format!("https://example.com/{}", "a".repeat(URL_LIMIT));
        assert_eq!(check("Site", &long), Err(LinkProblem::UrlTooLong));
        // Characters, not bytes: an accent is one.
        assert!(check(&"á".repeat(LABEL_LIMIT), "example.com").is_ok());
    }

    #[test]
    fn keeps_a_list_per_account_and_writes_it() {
        let file = std::env::temp_dir().join(format!(
            "votecord-profile-links-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&file);
        let mut store = Store::open(file.clone());
        let links = vec![check("Site", "example.com").unwrap()];
        store.accounts.insert("1".into(), links.clone());
        store.write().unwrap();
        let again = Store::open(file.clone());
        assert_eq!(again.accounts.get("1"), Some(&links));
        assert_eq!(again.accounts.get("2"), None);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "only this user reads it");
        }
        std::fs::remove_file(&file).unwrap();
    }

    #[test]
    fn a_missing_or_broken_file_holds_none() {
        let missing = std::env::temp_dir().join("votecord-profile-links-missing.json");
        let _ = std::fs::remove_file(&missing);
        assert!(Store::open(missing).accounts.is_empty());
        let broken = std::env::temp_dir().join(format!(
            "votecord-profile-links-broken-{}.json",
            std::process::id()
        ));
        std::fs::write(&broken, b"{not json").unwrap();
        assert!(Store::open(broken.clone()).accounts.is_empty());
        std::fs::remove_file(&broken).unwrap();
    }

    #[test]
    fn only_this_accounts_card_shows_them() {
        let account = Some("1");
        assert_eq!(for_profile(account, "1"), load("1"));
        assert!(for_profile(account, "2").is_empty());
        assert!(for_profile(None, "1").is_empty());
    }
}
