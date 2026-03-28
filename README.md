<div align="center">
    <img src="https://github.com/RoRvzzz/RustStrap/raw/main/interface/public/banner_black.png#gh-light-mode-only" width="820">
    <img src="https://github.com/RoRvzzz/RustStrap/raw/main/interface/public/banner_white.png#gh-dark-mode-only" width="820">

[![License][badge-repo-license]][repo-license]
[![Downloads][badge-repo-downloads]][repo-releases]
[![Version][badge-repo-latest]][repo-latest]
[![Discord][badge-discord]][discord-invite]
![Stars][badge-repo-stars]
</div>

> [!ANNOUCMENT]
> Please star the repo if you enjoy it!

> [!CAUTION]
> The only official place to download Ruststrap is this GitHub repository.
> Do not download binaries from mirrors, reupload sites, or unofficial domains.

> [!WARNING]
> Ruststrap is still in very early beta and is very buggy
> It is very prone to bugs in its current state


*Ruststrap is a bootstrapper based on Froststrap and Fishstrap (which are both based on bloxstrap)*
*Ruststrap is written entirely in rust using a tauri interface*

***Found a bug? [Open an issue](https://github.com/RoRvzzz/RustStrap/issues/new/choose) or report it in our [Discord server](https://discord.gg/macorstack).***

> [!NOTE]
> Ruststrap currently supports **Windows 10 and above**.
> Other operating systems are not in scope right now.

**Download the latest release [here][repo-latest]**

## feature list
- detailed server information
- server browser
- tray history
- server info
- discord rich presence with join/game-page buttons
- roblox studio support
- fast flag editor
- region selector (cookie + server enrichment flow)
- cache cleaner
- channel changer
- detached watcher + tray host flow

> and much more.

## build from source

### prerequisites
- [node.js](https://nodejs.org/)
- [rust](https://rustup.rs/)
- windows msvc build tools

### build
```bash
git clone https://github.com/RoRvzzz/RustStrap.git
cd RustStrap/interface
npm install
npm run tauri build
```

[badge-repo-license]:    https://img.shields.io/github/license/RoRvzzz/RustStrap?style=flat-square
[badge-repo-downloads]:  https://img.shields.io/github/downloads/RoRvzzz/RustStrap/latest/total?style=flat-square&color=ff7a1a
[badge-repo-latest]:     https://img.shields.io/github/v/release/RoRvzzz/RustStrap?style=flat-square&color=ff4d4f
[badge-repo-stars]:      https://img.shields.io/github/stars/RoRvzzz/RustStrap?style=flat-square&color=dd9900
[badge-discord]:         https://img.shields.io/discord/1364660238963179520?style=flat-square&logo=discord&logoColor=white&logoSize=auto&label=discord&color=5865f2

[repo-license]:  https://github.com/RoRvzzz/RustStrap/blob/main/LICENSE
[repo-releases]: https://github.com/RoRvzzz/RustStrap/releases
[repo-latest]:   https://github.com/RoRvzzz/RustStrap/releases/latest
[discord-invite]: https://discord.gg/macrostack
