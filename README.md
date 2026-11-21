<p align="center"><img src="design/medore/medore-icon-dark.svg" width="112" alt="MEDORE"></p>

<h1 align="center">MEDORE</h1>
<p align="center">A media player for Windows that plays everything, finds subtitles by itself and keeps your library tidy.</p>

## Download

Get the latest **MEDORE_x.y.z_x64-setup.exe** from [Releases](../../releases/latest) and run it.
It installs for your user only (no administrator rights) and updates itself
when a new version is published.

Windows may say *"Windows protected your PC"* the first time, because the
installer is not code-signed yet: click **More info → Run anyway**.

## What it does

- **Plays every format** — MKV, MP4, AVI, HEVC/H.265, AV1, VP9, 10-bit, HDR10, Dolby/DTS audio, FLAC… using libmpv with hardware decoding.
- **Subtitles by themselves** — finds subtitle files next to the video even when the names differ, in `Subs` folders and inside `.zip` files; reads old Persian Windows-1256 files correctly and fixes Arabic ي/ك to Persian ی/ک. Default font: Calibri.
- **Picks the right tracks** — your subtitle and audio languages, in order.
- **Library** — movies with posters, TV shows grouped into seasons and episodes, personal videos by folder, music by album; resume where you stopped; *Next up*.
- **Queue in natural order** — Episode 2 before Episode 10; sort, drag to reorder, shuffle.
- **Player** — timeline previews, chapters, speed, A-B loop, screenshots, picture in picture, always on top, media info, full keyboard control (F1).
- **Persian and English** interface, right-to-left done properly.
- **Icon follows Windows** — dark icon in dark mode, light icon in light mode.
- **Private** — no account and no tracking. The only thing sent online is a movie or show title to fetch its poster (can be turned off).

## Build from source

Requirements: Windows 10/11, Node.js 20+, Rust (GNU or MSVC toolchain), and
`libmpv-2.dll` in `vendor/mpv/` (a shinchiro mpv-dev build).

```powershell
npm install
. .\scripts\env.ps1          # Rust + MinGW on PATH (GNU toolchain)
.\scripts\dev-run.ps1        # debug build with a throwaway profile
cd src-tauri; cargo test -p medore-core
.\scripts\release.ps1        # installer + signed update files
```

## Licence

GNU GPL v3 or later (libmpv as distributed is GPL). Icons: Lucide (ISC).
Persian font: Vazirmatn (OFL).
