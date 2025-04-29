# Yato

A cli application to stream anime with [Anilist](https://anilist.co/) integration and Discord RPC written in rust.

*The application is named after the protagonist of Noragami: [Yato](https://noragami.fandom.com/wiki/Yato)*

## Features
- Stream anime online
- Update anime in Anilist after completion
- Skip anime __Intros__, __Outros__ and __Recaps__
- Skip __Filler__ episodes
- Discord presence
- Local anime history to continue from where you left off last time
- Configurable through config file


## Installing and Setup
> **Note**: `Yato` requires `mpv` and only `mpv`

### Linux



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
          Must be used with --anime.
  -d, --discord
          Enables/Disables Discord Rich Presence
      --change-token
          Deletes your auth token stored
      --new
          Allows the user to add a new anime
      --completion-time <PERCENTAGE>
          Allows user to set a different completion time
      --score-on-completion
          Toggles scoring on completion
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
  yato --dub --discord-presence
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

more settings can be found at config file.
config file is located at ```~/.config/yato/yato.conf```

```yaml    
#Please do not remove any setting, because it will break the app.

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
