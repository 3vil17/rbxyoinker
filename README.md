<p align="center">
  <img src="src/web/logo.png" width="96" alt="rbxyoinker logo" />
</p>

# rbxyoinker

Roblox asset downloader that runs locally. Start the exe, it opens in your browser,
paste an asset id or link, and it saves the file. Everything goes straight from your
machine to Roblox, there's no server in between.

Works with audio, images, decals, meshes, models, animations and pretty much anything
else that has an asset id.

## Installation

Download `rbxyoinker.exe` from [releases](https://github.com/3vil17/rbxyoinker/releases) and run it.
Windows only for now.

Or build it (Rust + MSVC build tools):

```
cargo build --release
```

The exe ends up in `target/release/rbxyoinker.exe`.

## Usage

When you run it, a browser tab opens. If it doesn't, go to http://127.0.0.1:48620.
Running the exe again while it's already open just brings the tab back. There's no
window, to close it use **Quit** at the bottom of the page.

- **Download** - paste an id or any roblox.com / create.roblox.com link and hit Fetch.
  You get a preview with the type, whether it's public or private, and a Play button
  for audio. Put a number in Version to grab an older version of the asset.
- **Bulk** - paste up to 1000 ids or links, or drop a `.rbxl` / `.rbxm` / `.rbxlx` /
  `.rbxmx` file on it to pull every asset id out of a place or model. Filter by type
  and by public / private before you start. Afterwards you can retry the failed ones,
  copy their ids or zip everything.
- **Migrate** - move a place's assets to your account or a group, from Studio. See below.
- **Accounts** - every cookie you added, with its avatar, whether it still works and
  how many requests it has done.
- **Settings** - cookies, download folder, sorting into folders per type, and
  saving meshes as `.obj` too.
- **History** - past downloads, searchable, with a button to open the file in Explorer.

## Migrate

Moves the assets a place uses to your account or a group you manage, and swaps the new
ids into the place. Useful when a game moves to a new group.

1. In the Migrate tab, click **Install plugin** and restart Studio.
2. Open your place, click **Migrate** in the Plugins tab, and pair it with the code from the app.
3. Click **Scan place**. The assets show up in the app.
4. Pick where they go and hit Move. Ctrl+Z in Studio undoes the swap.

Sounds, images, meshes and animations can be moved. **Check first** shows what can be
reached and how much of your monthly audio limit is left, without uploading anything.
After a run, **Save report** lists what moved and which old ids are still in the place.
The tab also has settings for upload speed, a name prefix, reusing earlier moves, keeping
local copies and what to do with ids inside scripts.

## Credentials

Most assets need you to be logged in, so add a `.ROBLOSECURITY` cookie in Settings.
It's stored in Windows Credential Manager, not in a file. Adding a few (alts are fine)
makes bulk downloads faster, requests get spread across them and if one gets rate
limited the others keep going.

Don't share your cookie with anyone, it's a full login to your account.

An Open Cloud API key (`legacy-asset:manage` scope) works too, but it's not
recommended: it reaches far fewer assets than a cookie, and it keeps working until you
delete it on create.roblox.com. It's tucked away at the bottom of Settings.

## How it gets private assets

If a direct download is refused, it tries again from inside games that might use the
asset: the creator's games, then for group assets the group owner's games, then games from
groups that person is in. Whatever works gets remembered for that creator. Bulk and
Migrate check whole lists against a game at once. If you know the game, put it in the
game box to try it first.

## When a download fails

It tells you what Roblox said for each thing it tried, for example `403 User is not
authorized to access Asset`. Usually it means the asset is private and none of your
accounts have access. Packages, moderated assets and deleted assets can't be downloaded.

## Disclaimer

rbxyoinker is a fan-made tool and isn't affiliated with or endorsed by Roblox. Only
download or reuse assets you made or have permission to use. Using your accounts with
third party tools is at your own risk.

## License

AGPL-3.0 with a few additional terms, see [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).

If you share a modified version or run it as a service, you have to publish your source
under the same license, keep the credit, say it's an unofficial modified version and give
it your own name and logo. Versions up to 0.2.8 were released under MIT.
