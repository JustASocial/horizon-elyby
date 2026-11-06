# Horizon Revo

[![Latest release](https://img.shields.io/github/v/release/JustASocial/horizon-elyby?display_name=tag&label=release)](https://github.com/JustASocial/horizon-elyby/releases/latest)
[![Windows build](https://img.shields.io/github/actions/workflow/status/JustASocial/horizon-elyby/msi.yml?branch=main&label=Windows%20build)](https://github.com/JustASocial/horizon-elyby/actions/workflows/msi.yml)
[![Downloads](https://img.shields.io/github/downloads/JustASocial/horizon-elyby/total)](https://github.com/JustASocial/horizon-elyby/releases)
[![GitHub stars](https://img.shields.io/github/stars/JustASocial/horizon-elyby?style=flat)](https://github.com/JustASocial/horizon-elyby/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/JustASocial/horizon-elyby?style=flat)](https://github.com/JustASocial/horizon-elyby/forks)
[![Open issues](https://img.shields.io/github/issues/JustASocial/horizon-elyby)](https://github.com/JustASocial/horizon-elyby/issues)
[![License](https://img.shields.io/github/license/JustASocial/horizon-elyby)](LICENSE)

**Play on online Minecraft servers with your Ely.by account through the Horizon launcher.** Horizon Revo signs in to Ely.by, passes that account's session to the game, and loads Ely.by skins. Online servers must support Ely.by authentication; Ely.by sessions do not authenticate against servers that only use Mojang's session service.

Horizon Revo is a Windows launcher for Horizon. Sign in with Ely.by, use a guest profile without registration, prepare game instances, manage mods, and launch the game from one place.

## Download

Open [Releases](https://github.com/JustASocial/horizon-elyby/releases/latest) and choose the package that fits:

- **Setup (.exe)** — a guided Windows installer that opens Horizon Revo when installation finishes.
- **MSI** — a Windows Installer package for managed or scripted installs; it also opens the launcher after a new installation.
- **Portable (.zip)** — extract the folder and run `Horizon Revo Portable.cmd`. Launcher settings and game files stay inside its `data` folder.

The installers are per-user and do not require administrator access. The portable package is for Windows x64.

## Features

- Ely.by account sign-in for compatible online servers, with session handoff to the Horizon game client.
- Guest profiles for offline play without registration.
- Ely.by skin lookup for account profile heads and in-game player textures.
- Create and manage game instances.
- Browse and manage mods.
- Download and launch Horizon builds.
- Portable mode with data stored alongside the launcher.
- Signed automatic launcher updates from the Horizon Revo repository. Game builds continue to come from Horizon.

## Ely.by skins

The launcher reads public skin textures from Ely.by. Use the **Ely.by** tab to open the official skin editor in your default browser. The browser keeps its own Ely.by sign-in; the launcher does not read or copy browser cookies. Ely.by's published skin-system API documents reading profiles and textures, but does not document skin uploads or changing the active skin, so those actions stay on Ely.by's site.

## Built with

Rust, GPUI Fast, WiX Toolset 5, GitHub Actions, GitHub Pages, and Cloudflare Workers.

## License

Horizon Revo is distributed under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components.
