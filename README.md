<p align="center">
  <img src="packaging/icon.svg" width="96" height="96" alt="">
</p>

<h1 align="center">Votecord</h1>

<p align="center">
  <a href="#get-started"><img src="https://img.shields.io/badge/Windows%20%7C%20Linux-0078D4" alt="Windows and Linux"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/votecord/votecord?color=8b5cf6" alt="License: GPL 3.0"></a>
</p>

<p align="center">
  <b>A Discord client without the web browser, in Votecord purple.</b><br>
  About 30 MB of RAM idle, where Discord's app takes 720 MB (<a href="#measured-against-discord">measured as Vesktop</a>).
</p>

<p align="center">
  Native app written in Rust · Voice, video and screen sharing<br>
  Slash commands that work · Nitro perks you turn on yourself
</p>

## What Votecord is

Votecord is a fork of [Yapfast](https://github.com/yapfast/yapfast), the
native Rust Discord client: no browser engine anywhere in it, talking to
Discord's API itself and drawing its own window with
[Slint](https://slint.dev) and Skia, like a game would. Everything Yapfast
does is here — servers, DMs, threads, forums, friends, voice, screen
sharing, GIFs — laid out the way Discord lays them out.

On top of that, Votecord changes three things:

### Nitro perks, off until you ask

Discord's Nitro extras that cost anything to show come **turned off**, so a
fresh Votecord spends the least it can:

- **Animated profile banners.** A banner Discord stores as a GIF (`a_…`
  hash) normally shows as its still PNG, which downloads and decodes the
  least. Settings → Nitro offers three ways of showing one, each of which
  you turn on yourself: **Off** (the default) keeps the still; **First
  second** loads the GIF and plays its first second, then holds the frame
  it reached still — the timer stops, so nothing wakes to draw another
  frame, and the GIF's decoders are swept soon after; **Always** plays it
  for as long as it shows, in profile cards and the DM profile panel, with
  the same focus-pausing every other animation follows. Turning one of
  these on points the banners on screen at the GIF straight away; turning
  them off goes back to stills just as fast.

Nothing here touches your Discord subscription; it's how much *Votecord*
spends showing what Discord already sent.

### Slash commands that work

Type `/` in the composer and the room's commands come up — a server's, a
DM's, or those installed on your account — with the app that offers each
one. Pick one, or keep typing to filter. The words after its name fill its
options in order, shown as chips above the composer: filled ones dimmed,
the one being filled outlined in purple, with its description underneath.
Quotes keep a value together (`/remind me "walk the dog" 2h`), a
subcommand comes first among the words, and whatever runs past the last
string option joins it, so long values need no quotes. Enter sends it as
Discord's own client does, and the reply arrives with the room's messages.

The picker's Commands tab (beside GIFs and Emoji) lists the room's
commands to browse rather than type: a search keeps the ones whose name or
description matches, and a chip per app — with All beside them — keeps one
app's commands showing. Choosing one fills the composer with it.

### Votecord purple

Where Yapfast marked selection, primary actions and mentions of you in
marigold, Votecord marks them in purple (`#8b5cf6`), with a new mark of
its own: a ballot tile holding a speech bubble that casts its check. The
Vote theme keeps Yapfast's layered graphite surfaces; all the other
built-in themes — Bonfire, Catppuccin, Dracula, Nord and the rest — are
unchanged, and the theme editor still makes your own.

## Measured against Discord

<table>
  <tr><th></th><th>Vesktop memory</th><th>Votecord memory</th><th>Vesktop CPU</th><th>Votecord CPU</th></tr>
  <tr><td>Idle</td><td>724 MB</td><td>30 MB (−96%)</td><td>0.11%</td><td>0.015% (−86%)</td></tr>
  <tr><td>Image-heavy channel</td><td>706 MB</td><td>41 MB (−94%)</td><td>0.17%</td><td>0.008% (−95%)</td></tr>
  <tr><td>In a call</td><td>888 MB</td><td>44 MB (−95%)</td><td>0.46%</td><td>0.33% (−28%)</td></tr>
  <tr><td>Sharing your screen¹</td><td>1,013 MB</td><td>113 MB (−89%)</td><td>2.15%</td><td>1.1% (−48%)</td></tr>
  <tr><th></th><th colspan="2">Discord</th><th colspan="2">Votecord</th></tr>
  <tr><td>Opening the app²</td><td colspan="2" align="center">8 s</td><td colspan="2" align="center">0.22 s (36× faster)</td></tr>
</table>

The numbers are Yapfast's, measured on 25 October 2026 on the same PC
(screen sharing on 27 October); Votecord is the same engine with the
changes above, which only add what's turned on. See
[the measurements](docs/measurements/README.md). Your numbers will depend
on how many servers and DMs you have.

## Features

- **Servers, channels, DMs and group DMs**, live, with member lists, roles,
  emoji, stickers and the rest of running a server.
- **Messages** with Markdown, embeds, custom emoji, spoilers, replies,
  forwards, pins and search.
- **Slash commands**, as above.
- **Threads, forums and polls**.
- **Voice, cameras and Go Live**: mute, deafen, per-person volume, noise
  suppression, push-to-talk, streams and the soundboard.
- **The GIF picker** whose favourites live on disk as small clips, there
  the moment it opens.
- **Friends**: requests, nicknames, notes, profile cards, statuses.
- **Notifications** that follow your Discord settings.
- **Themes**: ten built in, or your own in the editor.
- **Games on your profile**, detected among running programs.
- **Updates offered in the app**: a bar names the newer release, and its
  button fetches the installer and runs it.

### Not there yet

- Media channels and Activities.
- Making and changing server folders.
- A server's onboarding form for new members.
- Slash commands' per-option pickers (user/channel lists); the options fill
  from the words you type, as above, and the picker's Commands tab browses
  the room's commands.

## Will Discord ban me?

> [!WARNING]
> Probably not, but nobody can promise it. Third-party clients, Votecord
> included, are against Discord's Terms of Service. In practice Discord
> rarely bans anyone for using one; the accounts that get banned are almost
> always spamming or running automation. If losing your account would be a
> disaster, don't use any third-party client, this one or another.

## Where your messages go

Straight from your PC to Discord, or to your Matrix server. No Votecord
server sits in between.

- You link your account by scanning a QR code with Discord's phone app, so
  Votecord never sees your password.
- The login token stays in your system's credential store, and only ever
  goes to Discord.
- It sends no analytics or crash reports.
- Updates are noticed on this repository's own releases, offered in a bar
  whose button fetches the installer; Settings → Advanced turns the checks
  off. Nothing is sent but the request itself.
- Pictures and GIFs load from wherever they're hosted, such as Discord or
  Tenor.

## Get started

**Windows:** download `Votecord-VERSION-windows-x64-setup.exe` from the [Releases page](https://github.com/Soyodk/VoteCord/releases), run it, and follow the installer. It installs for your Windows user without an administrator prompt, creates Start Menu shortcuts, and lets you enable startup at sign-in. The portable ZIP is also published there.

Votecord builds from source, in the same pinned environment Yapfast does:

```sh
# Try the UI without an account
nix develop path:./nix --command cargo run --locked -- --preview

# Run it for real, with Discord and Matrix
nix develop path:./nix --command cargo run --locked

# Discord only, like the release
nix develop path:./nix --command cargo run --locked --no-default-features --features desktop,discord

# Format, lint and test everything
scripts/check --fast
```

Without Nix, `cargo run --locked` works with rustup (Rust 1.97.1 from
`rust-toolchain.toml`) and the libraries the Arch package builds with, in
[packaging/arch/PKGBUILD](packaging/arch/PKGBUILD). See
[Building](docs/building.md) for the whole story.

| Read this | For |
| --- | --- |
| [Contributing](CONTRIBUTING.md) | What a pull request needs |
| [Development](docs/development.md) | The preview, local test servers, voice tests |
| [Architecture](docs/architecture.md) | How `votecord-core` and the Slint client fit together |
| [Discord](docs/discord/README.md) | What's done, what's next |

## Credits and licence

Votecord stands on [Yapfast](https://github.com/yapfast/yapfast) by
wiredtenshi and its contributors, who did the hard parts; this fork thanks
them. It's free software under the [GNU General Public License](LICENSE),
version 3 or later, as Yapfast is.

It speaks to Discord the way its own apps do, as documented by
[Discord Userdoccers](https://docs.discord.food), and stands on
[Slint](https://slint.dev), [Skia](https://skia.org), [LiveKit](https://livekit.io)
and WebRTC.

<sub>Votecord isn't affiliated with or endorsed by Discord Inc. Discord is a
trademark of Discord Inc.</sub>
