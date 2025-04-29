# Yato

A CLI application to stream anime with [Anilist](https://anilist.co/) integration and Discord Rich Presence, written in Rust.

*The application is named after the protagonist of Noragami: [Yato](https://noragami.fandom.com/wiki/Yato)*
<p align="center"><img alt="GitHub release" src="https://img.shields.io/github/v/release/Nandor206/yato"> <img alt="License" src="https://img.shields.io/github/license/Nandor206/yato"> <img alt="Made with Rust" src="https://img.shields.io/badge/made%20with-Rust-orange?logo=rust&logoColor=white"></p>

## Features
- Stream anime online
- Update anime in Anilist after completion
- Skip anime __intros__, __outros__ and __recaps__
- Skip __filler__ episodes
- Per-anime skip overrides — configure global skip settings (e.g. skip all intros), but customize behavior for specific anime by toggling them individually
- Discord presence
- Local anime history to continue from where you left off last time
- Configurable through config file

## 📸 Showcase

Here's a glimpse of what using Yato looks like:

| Watching ReLIFE, with Discord Presence on | Updating the overrides of Tokyo Ghoul Root A |
|-----------------|---------------|
| ![during-anime](screenshots/during-anime.png) | ![skip-overriding](screenshots/skip-override.png) |

## Installing and Setup
> **Note**: Yato requires [MPV](https://mpv.io) as the video player (support for others like VLC may be added in the future).

### Linux

```bash
curl -Lo curd https://github.com/Nandor206/yato/releases/latest/download/yato

chmod +x yato
sudo mv yato /usr/bin/
yato
```
> ##### Tested only on Fedora 42.
> If you're using macOS and can compile Yato, I'd appreciate it if you could share the binary — it would help others install it more easily too!

### Options
```
Usage: yato [OPTIONS] [QUERY]
Arguments:
[QUERY]   Watch specific anime without syncing with Anilist.
          Must be used with --number.
Options:
  -e, --edit
          Edit your config file in nano
  -c, --continue
          Continue watching from currently watching list (using the user's anilist account)
      --dub
          Allows user to watch anime in dub
      --sub
          Allows user to watch anime in sub
  -l, --language <LANGUAGE>
          Set preferred language (e.g. english, japanese, hungarian, etc.) - default: english [aliases: lang]
  -q, --quality <QUALITY>
          Specify the video quality (e.g. 1080p, 720p, etc. — default: best available).
  -i, --information <ANILIST ID OR NAME>
          Displays information of the anime [aliases: info]
  -n, --number <EPISODE NUMBER>
          Specify the episode number to start watching from.
          Must be used with a [QUERY].
  -d, --discord
          Enables/Disables Discord Rich Presence
      --change-token
          Deletes your auth token stored
      --new
          Allows the user to add a new anime
      --completion-time <PERCENTAGE>
          Allows user to set a different completion time
      --score-on-completion
          Toggle whether to set a score when the anime is marked as completed
      --skip-op
          Toggles the setting set in the config
      --skip-ed
          Toggles the setting set in the config
      --skip-filler
          Toggles the setting set in the config
      --skip-recap
          Toggles the setting set in the config

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
- **Note**:
    Most options can be specified in the config file as well.
    Options that are use are a toggle of the setting set in the config file.

### Examples

- **Continue Anime in dub with discord presence**:
  ```bash
  yato --dub --discord
  ```

- **Add a New Anime**:
  ```bash
  yato --new
  ```

- **Play with skipping off (if using the default settings)**:
  ```bash
  yato --skip-op --skip-ed --skip-re
  ```

## Configuration

All configurations are stored in a file you can edit with the `-e` option.

```bash
yato -e
```

More settings can be found in the configuration file located at: 
```~/.config/yato/yato.conf```

```yaml    
#Please do not remove any setting, because it will break the app, just leave it as is.

player: "mpv"
player_args: ""
# Player arguments, you can add any argument here. For example: "--no-cache --fullscreen=yes"
show_adult_content: false

score_on_completion: false
completion_time: 85
# You can change this to any number between 0 and 100.

skip_opening: true
skip_credits: true
skip_recap: true
skip_filler: false

quality: "best"
# You can change this to any other quality. If desired quality is not available, the app will choose the best available quality.

language: "english"
# Supported languages rn: hungarian, english. Hungarian uses a custom scraper for links (made by me)
sub_or_dub: "sub"
# This setting is currently only available for english. Needs to be "sub" or "dub"

discord_presence: false 
```
## Dependencies
- mpv - Video player (vlc support might be added later)
    
## APIs Used
#### [Anilist API](https://docs.anilist.co/) - For updating, fetching user and anime data.
#### [AniSkip API](https://api.aniskip.com/api-docs) - Get anime intro, outro and recap timings
#### [Jikan](https://jikan.moe/) - Get filler episode number

## Credits for url scraping:
#### [ani-cli](https://github.com/pystardust/ani-cli) - Code for fetching english anime urls

## Credits for the inspiration:
#### [jerry](https://github.com/justchokingaround/jerry), [curd](https://github.com/Wraient/curd)

---

> Made with 🦀 and ☕ by a fan of Noragami.  
> Want to contribute? Pull requests and feedbacks are welcome!
