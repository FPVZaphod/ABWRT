use std::{
    fs, io,
    path::{Path, PathBuf},
};

const FOLDER_SUFFIX: &str = "_osd";
const FRAME_NUMBER_DIGITS: usize = 6;
const FRAME_EXTENSION: &str = "png";

/// Prefix of the frame file names, e.g. `DJI_0001_osd` for `DJI_0001.mp4`.
pub fn png_file_prefix(input_video: &Path) -> String {
    let stem = input_video
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".into());
    format!("{stem}{FOLDER_SUFFIX}")
}

/// Folder the PNG sequence is written to. This is a sub folder named after the input video that is
/// created inside `parent`, or next to the input video if no parent is given.
pub fn png_sequence_dir(input_video: &Path, parent: Option<&Path>) -> PathBuf {
    let parent = parent
        .map(Path::to_path_buf)
        .or_else(|| input_video.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    parent.join(png_file_prefix(input_video))
}

/// ffmpeg output pattern for the frames, e.g. `<dir>/DJI_0001_osd_%06d.png`.
pub fn png_output_pattern(dir: &Path, input_video: &Path) -> PathBuf {
    dir.join(format!(
        "{}_%0{}d.{}",
        png_file_prefix(input_video),
        FRAME_NUMBER_DIGITS,
        FRAME_EXTENSION
    ))
}

/// Frames from a previous render that would be overwritten or left behind by a new render.
/// Only files that match the naming scheme of this tool are returned, other files are never touched.
pub fn existing_png_frames(dir: &Path, input_video: &Path) -> io::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let prefix = format!("{}_", png_file_prefix(input_video));
    let mut frames = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let is_frame = path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(FRAME_EXTENSION))
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(&prefix))
                .is_some_and(|n| n.len() == FRAME_NUMBER_DIGITS && n.chars().all(|c| c.is_ascii_digit()));
        if is_frame {
            frames.push(path);
        }
    }
    frames.sort();
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_defaults_to_next_to_video() {
        let dir = png_sequence_dir(Path::new("/videos/DJI_0001.mp4"), None);
        assert_eq!(dir, PathBuf::from("/videos/DJI_0001_osd"));
    }

    #[test]
    fn dir_uses_custom_parent() {
        let dir = png_sequence_dir(Path::new("/videos/DJI_0001.mp4"), Some(Path::new("/out")));
        assert_eq!(dir, PathBuf::from("/out/DJI_0001_osd"));
    }

    #[test]
    fn pattern_has_six_digit_frame_number() {
        let pattern = png_output_pattern(Path::new("/out/DJI_0001_osd"), Path::new("/videos/DJI_0001.mp4"));
        assert_eq!(pattern, PathBuf::from("/out/DJI_0001_osd/DJI_0001_osd_%06d.png"));
    }

    #[test]
    fn existing_frames_only_matches_own_files() {
        let tmp = std::env::temp_dir().join(format!("png_output_test_{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        for name in [
            "DJI_0001_osd_000001.png",
            "DJI_0001_osd_000002.png",
            "DJI_0001_osd_notes.png",
            "other_000001.png",
            "DJI_0001_osd_000003.txt",
        ] {
            fs::write(tmp.join(name), b"").unwrap();
        }
        let frames = existing_png_frames(&tmp, Path::new("/videos/DJI_0001.mp4")).unwrap();
        fs::remove_dir_all(&tmp).unwrap();
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn existing_frames_of_missing_dir_is_empty() {
        let frames = existing_png_frames(Path::new("/does/not/exist"), Path::new("a.mp4")).unwrap();
        assert!(frames.is_empty());
    }
}
