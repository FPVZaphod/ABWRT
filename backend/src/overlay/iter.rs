use std::{iter::Peekable, vec::IntoIter};

use crossbeam_channel::{Receiver, Sender};
use ffmpeg_sidecar::{
    child::FfmpegChild,
    event::{FfmpegEvent, OutputVideoFrame},
    iter::FfmpegIterator,
};
use image::{Rgba, RgbaImage};

use super::overlay_osd;
use crate::{
    ffmpeg::{handle_decoder_events, FromFfmpegMessage, ToFfmpegMessage},
    font,
    osd::{self, OsdOptions},
};

/// What the OSD is drawn on top of.
#[derive(Debug, Clone, Copy)]
pub enum Background {
    /// The decoded video frame.
    Video,
    /// A solid color so the OSD can be keyed out in a video editor.
    ChromaKey([f32; 3]),
    /// A fully transparent image.
    Transparent,
}

pub struct FrameOverlayIter {
    decoder_iter: FfmpegIterator,
    decoder_process: FfmpegChild,
    osd_frames_iter: Peekable<IntoIter<osd::Frame>>,
    font_file: font::FontFile,
    osd_options: OsdOptions,
    current_osd_frame: osd::Frame,
    ffmpeg_sender: Sender<FromFfmpegMessage>,
    ffmpeg_receiver: Receiver<ToFfmpegMessage>,
    background: BackgroundImage,
}

enum BackgroundImage {
    Video,
    Solid(Rgba<u8>),
}

impl FrameOverlayIter {
    #[tracing::instrument(skip(decoder_iter, decoder_process, osd_frames, font_file), level = "debug")]
    pub fn new(
        decoder_iter: FfmpegIterator,
        decoder_process: FfmpegChild,
        osd_frames: Vec<osd::Frame>,
        font_file: font::FontFile,
        osd_options: &OsdOptions,
        ffmpeg_sender: Sender<FromFfmpegMessage>,
        ffmpeg_receiver: Receiver<ToFfmpegMessage>,
        background: Background,
    ) -> Self {
        let mut osd_frames_iter = osd_frames.into_iter();
        let first_osd_frame = osd_frames_iter.next().unwrap();
        let background = match background {
            Background::Video => BackgroundImage::Video,
            Background::ChromaKey(c) => BackgroundImage::Solid(Rgba([
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                255,
            ])),
            Background::Transparent => BackgroundImage::Solid(Rgba([0, 0, 0, 0])),
        };
        Self {
            decoder_iter,
            decoder_process,
            osd_frames_iter: osd_frames_iter.peekable(),
            font_file,
            osd_options: osd_options.clone(),
            current_osd_frame: first_osd_frame,
            ffmpeg_sender,
            ffmpeg_receiver,
            background,
        }
    }
}

impl Iterator for FrameOverlayIter {
    type Item = OutputVideoFrame;

    fn next(&mut self) -> Option<Self::Item> {
        //  On every iteration check if the render should be stopped
        while let Ok(ToFfmpegMessage::AbortRender) = self.ffmpeg_receiver.try_recv() {
            self.decoder_process.quit().unwrap();
        }

        self.decoder_iter.find_map(|e| match e {
            FfmpegEvent::OutputFrame(mut video_frame) => {
                // For every video frame check if frame time is later than the next OSD frame time.
                // If so advance the iterator over the OSD frames so we use the correct OSD frame
                // for this video frame
                if let Some(next_osd_frame) = self.osd_frames_iter.peek() {
                    let next_osd_frame_secs = next_osd_frame.time_millis as f32 / 1000.0;
                    if video_frame.timestamp > next_osd_frame_secs * self.osd_options.osd_playback_speed_factor {
                        self.current_osd_frame = self.osd_frames_iter.next().unwrap();
                    }
                }

                let mut frame_image = match self.background {
                    BackgroundImage::Solid(color) => {
                        RgbaImage::from_pixel(video_frame.width, video_frame.height, color)
                    }
                    BackgroundImage::Video => {
                        RgbaImage::from_raw(video_frame.width, video_frame.height, video_frame.data).unwrap()
                    }
                };

                overlay_osd(
                    &mut frame_image,
                    &self.current_osd_frame,
                    &self.font_file,
                    &self.osd_options,
                );

                video_frame.data = frame_image.as_raw().to_vec();
                Some(video_frame)
            }
            other_event => {
                handle_decoder_events(other_event, &self.ffmpeg_sender);
                None
            }
        })
    }
}
