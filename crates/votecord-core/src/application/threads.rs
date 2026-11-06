//! A Discord channel's list of its threads, as its header's Threads button
//! opens it in Discord's client: the active threads this account is in,
//! the other active ones, then older, archived ones, a page at a time, or
//! the threads a search of their names finds. The active ones are the
//! server's, as its snapshot shares them; only the archived pages and a
//! search's results bring their own. All of it is kept only while the
//! list shows.

use super::*;
use crate::chat::{ListedThread, ThreadPerson};

/// How many archived threads one page asks for, as Discord's client does.
const PAGE: u32 = 25;

/// A channel's list of its threads, as the header's Threads button opens it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadList {
    /// The channel whose threads this is.
    pub room: String,
    /// Older, archived threads, a page at a time.
    pub archived: Vec<ListedThread>,
    /// Whether more archived threads remain.
    pub more: bool,
    /// The offset of the next page, or zero while none has been asked for.
    pub next: u32,
    /// A page is on its way.
    pub loading: bool,
    /// Why the last page failed, if it did.
    pub error: Option<String>,
    /// A search of the names, while one shows.
    pub search: Option<ThreadSearch>,
}

/// A search of the threads' names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThreadSearch {
    pub query: String,
    /// None while on its way, or why it failed, or the threads it found.
    pub found: Option<Result<Vec<ListedThread>, String>>,
}

/// The three sections a list of threads shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadSections {
    /// Active threads this account is in.
    pub joined: Vec<ListedThread>,
    /// Other active threads.
    pub other: Vec<ListedThread>,
    /// Older, archived ones.
    pub older: Vec<ListedThread>,
}

impl ThreadList {
    /// Opens the list for a channel, by room id: asks for the first page of
    /// archived threads if the account may read the history.
    pub fn open(room: String, archived: bool) -> (Self, Vec<Effect>) {
        let mut list = Self {
            room,
            more: archived,
            ..Self::default()
        };
        let effects = if archived {
            list.loading = true;
            list.more()
        } else {
            vec![]
        };
        (list, effects)
    }

    /// Asks for the next page of archived threads, if any remain and none is
    /// on its way.
    pub fn more(&mut self) -> Vec<Effect> {
        if !self.more || self.loading || self.search.is_some() {
            return vec![];
        }
        self.loading = true;
        self.error = None;
        vec![Effect::Backend(Command::Discord(DiscordCommand::Threads {
            room: self.room.clone(),
            offset: self.next,
        }))]
    }

    /// Searches the names, as Enter in the list's search does; an empty query
    /// shows the threads again.
    pub fn search(&mut self, query: String) -> Vec<Effect> {
        if query.is_empty() {
            self.search = None;
            return vec![];
        }
        self.search = Some(ThreadSearch {
            query: query.clone(),
            found: None,
        });
        vec![Effect::Backend(Command::Discord(DiscordCommand::SearchThreads {
            room: self.room.clone(),
            query,
        }))]
    }

    /// Takes a page of archived threads, if it's the one it asked for.
    pub fn came(
        &mut self,
        query: Option<String>,
        offset: u32,
        result: Result<(Vec<ListedThread>, bool), String>,
    ) {
        match query {
            Some(query) => {
                if let Some(search) = self.search.as_mut().filter(|s| s.query == query) {
                    search.found = Some(result.map(|(threads, _)| threads));
                }
            }
            None if offset == self.next => {
                self.loading = false;
                match result {
                    Ok((threads, more)) => {
                        self.archived.extend(threads);
                        self.more = more;
                        self.next += PAGE;
                        self.error = None;
                    }
                    Err(error) => {
                        self.error = Some(error);
                    }
                }
            }
            None => {}
        }
    }
}

impl Application {
    /// Shows a channel's list of its threads, by room id, in place of messages.
    pub(super) fn show_threads(&mut self, room: String) -> Vec<Effect> {
        let archived = self.may(&room, chat::Permission::ReadHistory);
        let (list, effects) = ThreadList::open(room, archived);
        self.state.threads = Some(list);
        effects
    }

    /// Forgets the list of threads, as another room shows.
    pub(super) fn close_threads(&mut self) -> Vec<Effect> {
        match self.state.threads.take() {
            Some(_) => vec![Effect::Backend(Command::Discord(
                DiscordCommand::CloseThreads,
            ))],
            None => vec![],
        }
    }

    /// Searches the threads' names, as Enter in the list's search does; an
    /// empty query shows the threads again.
    pub(super) fn search_threads(&mut self, query: String) -> Vec<Effect> {
        match self.state.threads.as_mut() {
            Some(list) => list.search(query),
            None => vec![],
        }
    }

    /// Wants another page of archived threads, as the list does when scrolled
    /// near its end.
    pub(super) fn more_threads(&mut self) -> Vec<Effect> {
        match self.state.threads.as_mut() {
            Some(list) => list.more(),
            None => vec![],
        }
    }

    /// Takes a page of threads for the list, if it's the one it asked for.
    pub(super) fn threads_came(
        &mut self,
        room: String,
        query: Option<String>,
        offset: u32,
        result: Result<(Vec<ListedThread>, bool), String>,
    ) -> Vec<Effect> {
        let Some(list) = self
            .state
            .threads
            .as_mut()
            .filter(|list| list.room == room)
        else {
            return vec![];
        };
        list.came(query, offset, result);
        vec![]
    }

    /// A list's threads by section, as [`Self::thread_sections`] gives
    /// them.
    pub(super) fn sections(&self, list: &ThreadList) -> ThreadSections {
        let mut sections = ThreadSections::default();
        let known = self.known_threads(&list.room);
        // Loaded pages reach back to the oldest of them; those the server
        // knows were archived after it join them. Until the first page
        // answers, none do, so they don't show and vanish again.
        let reach = match (list.next, list.more) {
            (0, _) => None,
            (_, true) => list.archived.iter().map(|thread| thread.last).min(),
            (_, false) => Some(0),
        };
        for thread in &known {
            let section = match (thread.archived, thread.joined) {
                (false, true) => &mut sections.joined,
                (false, false) => &mut sections.other,
                (true, _) if reach.is_some_and(|reach| thread.last >= reach) => &mut sections.older,
                (true, _) => continue,
            };
            section.push(thread.clone());
        }
        for thread in &list.archived {
            if !known.iter().any(|known| known.room == thread.room) {
                sections.older.push(thread.clone());
            }
        }
        for section in [
            &mut sections.joined,
            &mut sections.other,
            &mut sections.older,
        ] {
            section.sort_by_key(|thread| std::cmp::Reverse(thread.last));
        }
        sections
    }

    /// Which threads this account may start in a Discord server's text or
    /// announcement channel, by room id, as Discord's client decides:
    /// public ones with Create Public Threads and Read Message History,
    /// private ones with Create Private Threads, in a text channel only.
    pub fn thread_starts(&self, room: &str) -> (bool, bool) {
        #[cfg(feature = "discord")]
        if let Some((server, Some(channel), _)) = self.discord_place(room)
            && let Some(channel) = server
                .text_channels
                .iter()
                .find(|known| known.id == channel)
        {
            let public = self.may(room, chat::Permission::CreatePublicThreads)
                && self.may(room, chat::Permission::ReadHistory);
            let private =
                !channel.announcement && self.may(room, chat::Permission::CreatePrivateThreads);
            return (public, private);
        }
        let _ = room;
        (false, false)
    }

    /// Whether a thread can start from the open channel's messages, which
    /// their menus offer.
    pub fn can_start_threads(&self) -> bool {
        self.state.discord_online
            && self
                .state
                .conversation_that(|can| can.threads)
                .is_some_and(|room| !room.direct && !room.voice && room.parent.is_none())
            && self
                .state
                .selected
                .as_deref()
                .is_some_and(|room| self.thread_starts(room).0 || self.thread_starts(room).1)
    }

    /// The threads of a channel, by room id, that the server's snapshot
    /// knows and this account may see.
    fn known_threads(&self, room: &str) -> Vec<ListedThread> {
        #[cfg(feature = "discord")]
        if let Some((server, Some(channel), _)) = self.discord_place(room)
            && let Some(snapshot) = &self.discord
        {
            let names = snapshot.names.read();
            return server
                .threads()
                .filter(|thread| {
                    thread.parent.as_deref() == Some(channel)
                        && thread.marks.access != votecord_discord::ready::Access::Hidden
                })
                .filter_map(|thread| {
                    discord::listed_thread(server, &names, thread.channel, thread.marks)
                })
                .collect();
        }
        let _ = room;
        vec![]
    }

    /// A fingerprint of the active threads a forum would show for `room`.
    /// The UI uses it with [`ForumView::revision`] so a ThreadUpdate that
    /// changes a reply count or pin still invalidates the forum row cache.
    pub fn forum_threads_revision(&self, room: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for thread in self.known_threads(room) {
            thread.hash(&mut hasher);
        }
        hasher.finish()
    }
}
