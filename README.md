# Horizon Revo

![Horizon Revo in-game screenshot](docs/media/sunset.webp)

[![Latest release](https://img.shields.io/github/v/release/JustASocial/horizon-elyby?display_name=tag&label=release)](https://github.com/JustASocial/horizon-elyby/releases/latest)
[![Windows build](https://img.shields.io/github/actions/workflow/status/JustASocial/horizon-elyby/msi.yml?branch=main&label=Windows%20build)](https://github.com/JustASocial/horizon-elyby/actions/workflows/msi.yml)
[![Downloads](https://img.shields.io/github/downloads/JustASocial/horizon-elyby/total)](https://github.com/JustASocial/horizon-elyby/releases)
[![License](https://img.shields.io/github/license/JustASocial/horizon-elyby)](LICENSE)

Horizon Revo is a Windows launcher for Horizon. Sign in with Microsoft or Ely.by, prepare a game instance, manage mods, and start playing from one place.

## Download

Open [Releases](https://github.com/JustASocial/horizon-elyby/releases/latest) and choose the package that fits:

- **Setup (.exe)** — a guided Windows installer.
- **MSI** — a Windows Installer package for managed or scripted installs.
- **Portable (.zip)** — extract the folder and run `Horizon Revo Portable.cmd`. Launcher settings and game files stay inside its `data` folder.

The installers are per-user and do not require administrator access. The portable package is for Windows x64.

## Features

- Microsoft and Ely.by account sign-in.
- Create and manage game instances.
- Browse and manage mods.
- Download and launch Horizon builds.
- Portable mode with data stored alongside the launcher.
- Automatic launcher updates from the upstream Horizon repository.

## Built with

Rust, GPUI Fast, WiX Toolset 5, GitHub Actions, GitHub Pages, and Cloudflare Workers.

## License

Horizon Revo is distributed under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components.
