<p align="center">
  <img src="src/web/logo.png" width="96" alt="rbxyoinker logo" />
</p>

<h1 align="center">rbxyoinker</h1>

<p align="center">
  A Roblox asset downloader that runs on your own PC.<br>
  Single assets, bulk downloads, private assets, mesh conversion and moving a game's assets to a new owner.
</p>

<p align="center">
  <a href="https://github.com/3vil17/rbxyoinker/releases/latest"><img src="https://img.shields.io/github/v/release/3vil17/rbxyoinker?label=download&color=3bea57" alt="Latest release" /></a>
  <a href="https://github.com/3vil17/rbxyoinker/actions/workflows/ci.yml"><img src="https://github.com/3vil17/rbxyoinker/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <img src="https://img.shields.io/badge/platform-windows-555" alt="Windows" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-555" alt="AGPL-3.0" /></a>
</p>

<p align="center">
  <a href="#getting-started">Getting started</a> ·
  <a href="#features">Features</a> ·
  <a href="#meshes">Meshes</a> ·
  <a href="#migrate">Migrate</a> ·
  <a href="#security">Security</a> ·
  <a href="#troubleshooting">Troubleshooting</a>
</p>

![The Download tab](docs/download.png)

## Features

| | |
|---|---|
| **Download** | Any asset with an id: audio, images, decals, meshes, models, animations, heads and clothing. Preview it first, pick an older version, or point it at a game that uses it. |
| **Bulk** | Up to 1000 ids or links at once, or every id inside a `.rbxl` / `.rbxm` / `.rbxlx` / `.rbxmx`. Filter by type, retry failures by reason, zip the lot. |
| **Private assets** | If a direct download is refused, it tries again from inside games that might use the asset. It reaches a lot more than the website does. |
| **Meshes** | Every mesh version, 1.00 to 7.00, saved as `.obj` or `.glb` with vertex colors, bones and skin weights. |
| **Animated heads** | Heads come with the real animated version too: the `MeshPart` with face controls, plus its mesh. |
| **Convert** | Drop `.mesh` files you already have and get `.obj` / `.glb` back. Works offline. |
| **Migrate** | Moves a place's assets to your account or a group and swaps the new ids in, straight from Studio. |
| **Accounts** | Add a few cookies (alts are fine) and bulk work gets spread across them. |
| **Updates** | One click, with signed releases. |

## Getting started

1. Download `rbxyoinker.exe` from the [latest release](https://github.com/3vil17/rbxyoinker/releases/latest). There's nothing to install.
2. Windows may warn that it's from an unknown publisher, because the exe isn't code signed. Click **More info**, then **Run anyway**.
3. A browser tab opens. If it doesn't, go to http://127.0.0.1:48620.
4. Add a `.ROBLOSECURITY` cookie in **Settings**. Most assets need a signed in account. Use an alt.
5. Paste an id or link in **Download** and press Fetch.

To close it, close the tab or use **Quit** at the bottom of the sidebar.

> Only download rbxyoinker from this repo. Copies from anywhere else may be modified.

## The tabs

| Tab | What it's for |
|---|---|
| **Download** | One asset. Shows a preview, its type and whether it's public. `Ctrl+V` anywhere on the tab pastes and fetches. |
| **Bulk** | Paste ids or links, or drop a place or model file. Bundles and outfits open into their items. |
| **Migrate** | Move a place's assets from Studio to a new owner. [More below](#migrate). |
| **Convert** | Turn `.mesh` files into `.obj` or `.glb`. |
| **Accounts** | Every cookie you added, whether it still works and how busy it's been. |
| **History** | Your last 200 downloads, with search, type filters and download again. |
| **Settings** | Cookies, download folder, file names, sorting by type, mesh and image formats. |

![The Bulk tab](docs/bulk.png)

## Meshes

Mesh handling comes from [rbxmesh](https://github.com/3vil17/rbxmesh), a separate open source library by the same author. It reads every known mesh version, including version 7, which uses Draco compression and which most other tools can't open.

| Format | What you get |
|---|---|
| `.obj` | Geometry, normals, UVs and vertex colors. Opens almost anywhere. |
| `.glb` | All of that plus the skeleton and skin weights, so layered clothing and bodies open in Blender already rigged. |

Turn either on in **Settings**, or use the **Convert** tab for files you already have.

![The Convert tab](docs/convert.png)

## Migrate

Moves the assets a place uses to your account or a group you manage, then swaps the new ids into the place. Useful when a game moves to a new group.

1. Click **Studio** at the bottom of the sidebar, then **Install plugin**, and restart Studio.
2. Open your place, click **Migrate** in the Plugins tab and pair it with the code from the app.
3. Click **Scan place**. The assets show up in the app.
4. Pick where they go. **Check first** shows what can be reached and how much of your audio limit is left, without uploading anything.
5. Press Move. `Ctrl+Z` in Studio undoes the swap.

Sounds, images, meshes and animations can be moved. **Save report** lists what moved and which old ids are still in the place. Assets you already moved get reused, so a game with several places doesn't upload them twice.

**Use an alt for uploads.** If Roblox moderation flags an upload, the account that uploaded it takes the strike, even when the asset goes to a group. When the uploading account looks like a main account (old, Premium, limiteds, groups it owns and so on), Migrate warns you, lists why, and won't start until you confirm.

Bulk and Migrate skip Roblox's own default assets, like the sky, the baseplate grid and built-in sounds and animations, and show how many were skipped. You can turn this off in Settings.

![The Migrate tab](docs/migrate.png)

## Studio plugin

Once the plugin is paired, the app and Studio work together outside of Migrate too:

- **Insert into Studio.** The box button on a model in Recent or History drops it in front of the camera.
- **Animations onto a rig.** Select a rig, then press the box button on an animation. It goes into the rig's AnimSaves, where Eclipse Animator and Roblox's Animation Editor pick it up. **Copy place animations onto rig** in the plugin does this for every animation the place uses.
- **Download selection.** Sends the asset ids of whatever you have selected in Studio to Bulk.
- **Save rig as .glb.** Saves the selected rig with its textures and animations as an animated `.glb` in the Blender folder of your downloads, for 3D viewers, Blender, Unity, Godot and other 3D apps. To edit animations and send them back to Studio, use RBXMonkey below.

Animating in Blender? Put the yoinked animation on your rig, then send it over with [RBXMonkey](https://github.com/Cautioned/Blender-Animations-Plugin).

## How it gets private assets

When a direct download is refused, rbxyoinker tries again from inside games that might use the asset: the creator's games first, then for group assets the owner's games, then games from groups that person is in. Whatever works is remembered for that creator. If you know the game, put it in the game box and it goes first.

## Accounts and cookies

A `.ROBLOSECURITY` cookie is a full login, so never share it. rbxyoinker keeps cookies in Windows Credential Manager, not in a file, and only ever sends them to Roblox.

Adding a few accounts makes bulk downloads faster. Requests are spread across them, and if one gets rate limited the others keep going.

An Open Cloud API key with the `legacy-asset:manage` scope works too, but it reaches far fewer assets, so it's tucked away at the bottom of Settings.

## Security

- **Local only.** The app only listens on your own PC. Other websites can't talk to it, and every request has to carry a key that only the app's own page knows. A new key is made every launch.
- **Signed updates.** Releases are built by GitHub Actions and signed with a key that isn't on GitHub. The app won't install an update unless the signature matches, so a hacked GitHub account can't push a bad update.
- **Important updates are enforced.** When a release fixes something serious, older versions stop working until they update.

To check a download yourself, grab `rbxyoinker.exe.minisig` from the same release and run:

```
minisign -Vm rbxyoinker.exe -P RWQIISBNpKRKRP5ILebS96X44Nh05784HVJdk71MBLHvoezrKP1rgv6t
```

Found a security problem? Open an issue, but leave out the details of how to exploit it.

## Troubleshooting

When a download fails, rbxyoinker says why in plain words, and **Details** shows what Roblox said at each step.

| What you see | What it usually means |
|---|---|
| Private, no account can reach it | None of your accounts has access, and no game it was tried through works. Add the game it's used in. |
| Moderated or content deleted | Roblox removed it. It can't be downloaded. |
| In review | Roblox hasn't approved it yet. Try again later. |
| Package | Packages only download for accounts that have access to them. |

## Build it yourself

You need Rust and the MSVC build tools.

```
cargo build --release
```

The exe ends up in `target/release/rbxyoinker.exe`.

## Disclaimer

rbxyoinker is a fan-made tool and isn't affiliated with or endorsed by Roblox. Only download or reuse assets you made or have permission to use. Using your accounts with third party tools is at your own risk.

## License

AGPL-3.0 with a few additional terms. See [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).

If you share a modified version or run it as a service, you have to publish your source under the same license, keep the credit, say it's an unofficial modified version, and give it your own name and logo.
