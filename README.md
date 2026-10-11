<h1 align="center">ABWRT - A Better Walksnail Render Tool</h1>

Cross-platform tool for rendering the flight controller OSD from the Walksnail Avatar HD FPV system on top of the goggle or VRX recording, or as a transparent PNG image sequence for video editing software.

ABWRT is based on [Walksnail OSD Tool](https://github.com/avsaase/walksnail-osd-tool) by Alexander van Saase and uses its own name, settings and app identifier, so both programs can be installed next to each other.

## Features
- Easy to use graphical user interface.
- Render the OSD as a PNG image sequence with transparent background (one file per video frame, optionally every second frame) for use in video editing software.
- Built-in font, no need to load a font file first.
- No SRT file needed, only the video and the OSD file.
- App bundle for macOS.
- Hardware-accelerated encoding powered by ffmpeg.
- Choose between H.264 and H.265 codecs (more can be added later).
- View basic information about the video, OSD and font files.
- Preview OSD frames before rendering.
- Automatically center the OSD or position it manually.
- Selectable output video bitrate (more encoder settings will be added later).
- Upscale output video to 1440p for higher quality when uploading to YouTube.
- Mask OSD items ([demo](https://imgur.com/u8xi2tX)).

## PNG sequence
Choose "PNG sequence" under "Rendering Options". The frames are written to a folder named after the video with the suffix `_osd` (for example `AvatarG0001_osd/AvatarG0001_osd_000001.png`), next to the video or in a folder of your choice. Import the folder as an image sequence in your video editor. Videos with a variable frame rate are rendered at the constant frame rate that is shown next to the setting, import the sequence with this frame rate (half of it when only every second frame is written) to keep it in sync with the video.

## Installation

### macOS
The app bundles are built by the workflow "Build macOS" of this repository. Open the *Actions* tab, select the latest successful run and download the artifact for your processor (`abwrt-macos-arm` for Apple Silicon, `abwrt-macos-intel` for Intel Macs). Unzip it and drag the app to your Applications folder.

<details>
<summary>Instructions for running the first time</summary>

The app is not signed with a "Developer ID Certificate". When you try to run it for the first time macOS may say that the app is damaged or that the developer cannot be verified. Remove the quarantine flag once in the terminal:

```
xattr -dr com.apple.quarantine "/Applications/ABWRT.app"
```

Signing and notarizing is prepared in the workflow, see [`_deploy/macos/SIGNING.md`](_deploy/macos/SIGNING.md).
</details>

### Windows
There is no Windows build yet. The installer and release workflows of the original project are included as a template.

### Linux
For now you need to build from source.

### Building from source
1. Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
2. Run `cargo install --git https://github.com/FPVZaphod/ABWRT.git abwrt`. The executable will be installed in `$HOME/.cargo/bin/` and added to your path.
3. To run the app you need the `ffmpeg` and `ffprobe` binaries in your `path` or placed next to the executable you just build.
4. Run the app with `abwrt`.

### Similar projects
- [avsaase/walksnail-osd-tool](https://github.com/avsaase/walksnail-osd-tool): The project this one is based on.
- [kirek007/ws-osd-py](https://github.com/kirek007/ws-osd-py): Python-based tool with GUI and CLI. Depending on your OS it can require some manual setup due to Python dependencies.
- [shellixyz/hd_fpv_video_tool](https://github.com/shellixyz/hd_fpv_video_tool): Rust-based CLI tool with support for with Walksnail and DJI. Mainly targets Linux and can be difficult to build from source on Windows and MacOS. Has some cool features like live playback of the DVR with OSD without rendering.

## License
GPL-3.0, see [LICENSE.md](LICENSE.md). Copyright of the original code remains with its author.

## Disclaimer
This project is not affiliated with Walksnail/Caddx.
