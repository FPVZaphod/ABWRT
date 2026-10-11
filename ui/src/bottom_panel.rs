use std::path::PathBuf;

use backend::ffmpeg::{existing_png_frames, start_video_render, OutputMode, ToFfmpegMessage};
use egui::{vec2, Align, Align2, Button, Color32, Layout, ProgressBar, RichText, Ui};

use super::{util::format_minutes_seconds, WalksnailOsdTool};
use crate::{render_status::Status, util::get_output_path};

impl WalksnailOsdTool {
    pub fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                self.start_stop_render_button(ui);
                self.render_progress(ui);
            });
            ui.add_space(2.0);
        });
    }

    fn start_stop_render_button(&mut self, ui: &mut Ui) {
        let button_size = vec2(110.0, 40.0);
        if self.render_status.is_not_in_progress() {
            if ui
                .add_enabled(
                    self.all_files_loaded(),
                    Button::new("Start render").min_size(button_size),
                )
                .on_disabled_hover_text("First load video, OSD and font files")
                .clicked()
            {
                tracing::info!("Start render button clicked");
                let existing_frames = self.existing_png_frames();
                if existing_frames.is_empty() {
                    self.start_render();
                } else {
                    // Ask before touching the results of an earlier render
                    self.png_overwrite_confirm = Some(existing_frames.len());
                }
            }
        } else {
            if ui.add(Button::new("Stop render").min_size(button_size)).clicked() {
                tracing::info!("Stop render button clicked");
                if let Some(sender) = &self.to_ffmpeg_sender {
                    sender
                        .send(ToFfmpegMessage::AbortRender)
                        .map_err(|_| tracing::warn!("Failed to send abort render message"))
                        .unwrap();
                    self.render_status.stop_render();
                }
            }
        }
    }

    /// PNG frames from an earlier render of the same video in the output folder (PNG mode only).
    fn existing_png_frames(&self) -> Vec<PathBuf> {
        if self.render_settings.output_mode != OutputMode::PngSequence {
            return Vec::new();
        }
        let Some(video_path) = &self.video_file else {
            return Vec::new();
        };
        let output_dir = get_output_path(video_path, &self.render_settings);
        existing_png_frames(&output_dir, video_path).unwrap_or_default()
    }

    /// Ask for confirmation to delete the PNG frames of an earlier render, then start the render.
    pub fn png_overwrite_window(&mut self, ctx: &egui::Context) {
        let Some(frame_count) = self.png_overwrite_confirm else {
            return;
        };
        let output_dir = self
            .video_file
            .as_ref()
            .map(|v| get_output_path(v, &self.render_settings))
            .unwrap_or_default();

        let mut start = false;
        let mut cancel = false;
        egui::Window::new("Output folder not empty")
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "The folder {} already contains {} PNG files from an earlier render. They will be deleted before the new frames are written. Other files in the folder are not touched.",
                    output_dir.display(),
                    frame_count
                ));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Delete and render").clicked() {
                        start = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if start {
            self.png_overwrite_confirm = None;
            let failed = self
                .existing_png_frames()
                .iter()
                .filter(|f| {
                    std::fs::remove_file(f)
                        .map_err(|e| tracing::error!("Failed to delete {f:?}: {e}"))
                        .is_err()
                })
                .count();
            if failed == 0 {
                self.start_render();
            } else {
                self.render_status.status = Status::Error {
                    progress_pct: 0.0,
                    error: format!("Failed to delete {failed} existing PNG files"),
                };
            }
        } else if cancel {
            self.png_overwrite_confirm = None;
        }
    }

    fn start_render(&mut self) {
        self.render_status.start_render();
        if let (Some(video_path), Some(osd_file), Some(font_file), Some(video_info)) =
            (&self.video_file, &self.osd_file, &self.font_file, &self.video_info)
        {
            self.osd_options.osd_playback_speed_factor = if self.osd_options.adjust_playback_speed {
                let video_duration = video_info.duration;
                let osd_duration = osd_file.duration;
                video_duration.as_secs_f32() / osd_duration.as_secs_f32()
            } else {
                1.0
            };
            match start_video_render(
                &self.dependencies.ffmpeg_path,
                video_path,
                &get_output_path(video_path, &self.render_settings),
                osd_file.frames.clone(),
                font_file.clone(),
                &self.osd_options,
                video_info,
                &self.render_settings,
            ) {
                Ok((to_ffmpeg_sender, from_ffmpeg_receiver)) => {
                    self.to_ffmpeg_sender = Some(to_ffmpeg_sender);
                    self.from_ffmpeg_receiver = Some(from_ffmpeg_receiver);
                }
                Err(e) => {
                    tracing::error!("Failed to start render: {e}");
                    self.render_status.status = Status::Error {
                        progress_pct: 0.0,
                        error: "Failed to start render".to_string(),
                    }
                }
            };
        }
    }

    fn render_progress(&mut self, ui: &mut Ui) {
        match &self.render_status.status {
            Status::Idle => {}
            Status::InProgress {
                time_remaining,
                fps,
                speed,
                progress_pct,
            } => {
                ui.vertical(|ui| {
                    ui.add(ProgressBar::new(*progress_pct).show_percentage());
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.add_space(3.0);
                        let time_remaining_string = if let Some(time_remaining) = time_remaining {
                            format_minutes_seconds(time_remaining)
                        } else {
                            "––:––".into()
                        };
                        ui.label(format!(
                            "Time remaining: {}, fps: {:.1}, speed: {:.3}x",
                            time_remaining_string, fps, speed
                        ));
                    });
                });
            }
            Status::Completed => {
                ui.vertical(|ui| {
                    ui.add(ProgressBar::new(1.0).text("Done"));
                });
            }
            Status::Cancelled { progress_pct } => {
                ui.vertical(|ui| {
                    ui.add(ProgressBar::new(*progress_pct).text("Cancelled"));
                });
            }
            Status::Error { progress_pct, error } => {
                ui.vertical(|ui| {
                    ui.add(ProgressBar::new(*progress_pct));
                    ui.label(RichText::new(error.clone()).color(Color32::RED));
                });
            }
        }
    }
}
