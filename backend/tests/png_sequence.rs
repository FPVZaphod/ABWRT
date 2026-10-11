//! End-to-end test of the PNG sequence output with a synthetic video, OSD and font.
//! Skipped when ffmpeg or ffprobe are not installed.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use backend::{
    ffmpeg::{
        existing_png_frames, ffmpeg_available, ffprobe_available, png_sequence_dir, start_video_render,
        FromFfmpegMessage, OutputMode, PngFrameRate, RenderSettings, VideoInfo,
    },
    font::FontFile,
    osd::{Frame, OsdOptions},
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FRAME_RATE: u32 = 10;
const SECONDS: u32 = 2;

fn create_test_video(path: &Path) -> bool {
    Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg(format!(
            "testsrc=size={WIDTH}x{HEIGHT}:rate={FRAME_RATE}:duration={SECONDS}"
        ))
        .args(["-pix_fmt", "yuv420p"])
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Font where every character is a opaque white square.
fn create_test_font(path: &Path) {
    let (char_width, char_height) = (36, 54);
    let image = image::RgbaImage::from_pixel(char_width, char_height * 256, image::Rgba([255, 255, 255, 255]));
    image.save(path).unwrap();
}

/// Frames in the binary format of the .osd file: 4 byte timestamp followed by a 53x20 grid of 2 byte glyph indices.
fn osd_frames() -> Vec<Frame> {
    (0..SECONDS * FRAME_RATE)
        .map(|i| {
            let mut bytes = vec![0u8; 4 + 53 * 20 * 2];
            bytes[..4].copy_from_slice(&(i * 1000 / FRAME_RATE).to_le_bytes());
            let grid_idx = 2 * 53 + 2; // grid position (2, 2)
            bytes[4 + grid_idx * 2..4 + grid_idx * 2 + 2].copy_from_slice(&65u16.to_le_bytes());
            Frame::try_from(bytes.as_slice()).unwrap()
        })
        .collect()
}

fn render_to_end(
    video: &Path,
    dir: &Path,
    font: FontFile,
    info: &VideoInfo,
    settings: &RenderSettings,
) -> Result<(), String> {
    let (_tx, rx) = start_video_render(
        &PathBuf::from("ffmpeg"),
        &video.to_path_buf(),
        &dir.to_path_buf(),
        osd_frames(),
        font,
        &OsdOptions::default(),
        info,
        settings,
    )
    .map_err(|e| e.to_string())?;

    let start = Instant::now();
    let mut decoder_done = false;
    while start.elapsed() < Duration::from_secs(60) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(FromFfmpegMessage::DecoderFinished) => decoder_done = true,
            Ok(FromFfmpegMessage::EncoderFinished) if decoder_done => return Ok(()),
            Ok(FromFfmpegMessage::DecoderFatalError(e) | FromFfmpegMessage::EncoderFatalError(e)) => return Err(e),
            _ => {}
        }
    }
    Err("timeout".into())
}

fn render_png_sequence(png_frame_rate: PngFrameRate) -> (Vec<PathBuf>, PathBuf) {
    let tmp = std::env::temp_dir().join(format!("png_sequence_test_{}_{:?}", std::process::id(), png_frame_rate));
    fs::create_dir_all(&tmp).unwrap();
    let video = tmp.join("clip.mp4");
    let font_path = tmp.join("font.png");
    assert!(create_test_video(&video));
    create_test_font(&font_path);

    let info = VideoInfo::get(&video, &"ffprobe".into()).unwrap();
    let font = FontFile::open(font_path).unwrap();
    let settings = RenderSettings {
        output_mode: OutputMode::PngSequence,
        png_frame_rate,
        ..Default::default()
    };
    let out_dir = png_sequence_dir(&video, None);
    assert_eq!(out_dir, tmp.join("clip_osd"));

    render_to_end(&video, &out_dir, font, &info, &settings).unwrap();
    (existing_png_frames(&out_dir, &video).unwrap(), tmp)
}

fn ffmpeg_installed() -> bool {
    let installed = ffmpeg_available(&"ffmpeg".into()) && ffprobe_available(&"ffprobe".into());
    if !installed {
        eprintln!("ffmpeg/ffprobe not available, skipping");
    }
    installed
}

#[test]
fn renders_one_transparent_png_per_video_frame() {
    if !ffmpeg_installed() {
        return;
    }
    let (frames, tmp) = render_png_sequence(PngFrameRate::Full);

    assert_eq!(frames.len() as u32, SECONDS * FRAME_RATE);
    assert_eq!(frames[0].file_name().unwrap(), "clip_osd_000001.png");

    let image = image::open(&frames[0]).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (WIDTH, HEIGHT));
    // Background is fully transparent, not the video
    assert_eq!(image.get_pixel(WIDTH - 1, HEIGHT - 1).0, [0, 0, 0, 0]);
    // The glyph at grid position (2, 2) is opaque and white
    let (cw, ch) = (24, 36); // 720p character size
    let glyph_pixel = image.get_pixel(2 * cw + cw / 2, 2 * ch + ch / 2);
    assert_eq!(glyph_pixel.0, [255, 255, 255, 255]);

    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn half_frame_rate_writes_every_second_frame() {
    if !ffmpeg_installed() {
        return;
    }
    let (frames, tmp) = render_png_sequence(PngFrameRate::Half);

    assert_eq!(frames.len() as u32, SECONDS * FRAME_RATE / 2);
    // Numbering stays gapless so editors recognize the sequence
    assert_eq!(frames[0].file_name().unwrap(), "clip_osd_000001.png");
    assert_eq!(frames[9].file_name().unwrap(), "clip_osd_000010.png");

    fs::remove_dir_all(&tmp).unwrap();
}
