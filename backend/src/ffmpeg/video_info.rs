use std::{path::PathBuf, time::Duration};

use ffprobe::FfProbe;

use super::error::VideoInfoError;

#[derive(Debug)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    /// Average frame rate of the video. Can differ from the rate ffmpeg decodes at for videos with a variable frame rate.
    pub frame_rate: f32,
    /// Constant frame rate at which ffmpeg decodes the video: frames are duplicated or dropped to follow it.
    /// This is the rate the rendered output needs to be played at to stay in sync with the video.
    pub output_frame_rate: f32,
    pub bitrate: u32,
    pub duration: Duration,
    pub total_frames: u32,
}

impl VideoInfo {
    #[tracing::instrument(ret)]
    pub fn get(file_path: &PathBuf, ffprobe_path: &PathBuf) -> Result<Self, VideoInfoError> {
        let info = ffprobe::ffprobe(file_path, ffprobe_path.to_path_buf())?;
        info.try_into()
    }
}

impl TryFrom<FfProbe> for VideoInfo {
    type Error = VideoInfoError;

    fn try_from(value: FfProbe) -> Result<Self, Self::Error> {
        let stream = value.streams.first().ok_or(VideoInfoError::NoStream)?;

        let width = stream.width.ok_or(VideoInfoError::NoFrameWidth)? as u32;
        let height = stream.height.ok_or(VideoInfoError::NoFrameHeight)? as u32;
        let frame_rate = parse_frame_rate(&stream.avg_frame_rate).ok_or(VideoInfoError::NoFrameRate)?;
        let output_frame_rate = output_frame_rate(parse_frame_rate(&stream.r_frame_rate), frame_rate);
        let bitrate = stream
            .bit_rate
            .as_ref()
            .and_then(|b| b.parse::<u32>().ok())
            .ok_or(VideoInfoError::NoBitrate)?;

        let duration = Duration::from_secs_f32(
            stream
                .duration
                .as_ref()
                .and_then(|s| s.parse::<f32>().ok())
                .ok_or(VideoInfoError::NoDuration)?,
        );

        let total_frames = (output_frame_rate * duration.as_secs_f32()) as u32;

        Ok(Self {
            width,
            height,
            frame_rate,
            output_frame_rate,
            bitrate,
            duration,
            total_frames,
        })
    }
}

/// Parses ffprobe frame rates of the form `60/1` or `141396/2395`.
fn parse_frame_rate(frame_rate: &str) -> Option<f32> {
    let (num, den) = frame_rate.split_once('/')?;
    let rate = num.parse::<f32>().ok()? / den.parse::<f32>().ok()?;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}

/// ffmpeg decodes to a constant frame rate that equals the `r_frame_rate` of the stream, unless that is
/// implausibly high (some containers report the time base), in which case it uses the average frame rate.
fn output_frame_rate(r_frame_rate: Option<f32>, avg_frame_rate: f32) -> f32 {
    match r_frame_rate {
        Some(rate) if rate <= 210.0 => rate,
        _ => avg_frame_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frame_rates() {
        assert_eq!(parse_frame_rate("60/1"), Some(60.0));
        assert!((parse_frame_rate("141396/2395").unwrap() - 59.04).abs() < 0.01);
        assert_eq!(parse_frame_rate("0/0"), None);
        assert_eq!(parse_frame_rate("garbage"), None);
    }

    #[test]
    fn output_rate_follows_r_frame_rate() {
        assert_eq!(output_frame_rate(Some(60.0), 59.04), 60.0);
        assert_eq!(output_frame_rate(Some(90000.0), 30.0), 30.0);
        assert_eq!(output_frame_rate(None, 29.97), 29.97);
    }
}
