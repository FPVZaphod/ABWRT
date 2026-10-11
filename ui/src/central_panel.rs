use std::time::Instant;

use backend::util::Coordinates;
use egui::{
    vec2, Button, CentralPanel, Checkbox, CollapsingHeader, Color32, CursorIcon, Grid, Image, Rect, RichText,
    ScrollArea, Sense, Slider, Stroke, Ui, Vec2,
};

use backend::ffmpeg::{png_sequence_dir, OutputMode, PngFrameRate};

use crate::{
    osd_preview::{calculate_horizontal_offset, calculate_vertical_offset},
    util::{separator_with_space, tooltip_text},
    WalksnailOsdTool,
};

impl WalksnailOsdTool {
    pub fn render_central_panel(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().spacing.slider_width = self.ui_dimensions.osd_position_sliders_length;

                self.osd_options(ui, ctx);

                separator_with_space(ui, 10.0);

                self.osd_preview(ui, ctx);

                separator_with_space(ui, 10.0);

                self.rendering_options(ui);
            });
        });
    }

    fn osd_options(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let mut changed = false;

        CollapsingHeader::new(RichText::new("OSD Options").heading())
            .default_open(true)
            .show_unindented(ui, |ui| {
                Grid::new("osd_options")
                    .min_col_width(self.ui_dimensions.options_column1_width)
                    .show(ui, |ui| {
                        ui.label("Horizontal position")
                            .on_hover_text(tooltip_text("Horizontal position of the flight controller OSD (pixels from the left edge of the video)."));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(Slider::new(&mut self.osd_options.position.x, -200..=700).text("Pixels"))
                                .changed();

                            if ui.button("Center").clicked() {
                                if let (Some(video_info), Some(osd_file), Some(font_file)) =
                                    (&self.video_info, &self.osd_file, &self.font_file)
                                {
                                    self.osd_options.position.x = calculate_horizontal_offset(
                                        video_info.width,
                                        osd_file
                                            .frames
                                            .get(self.osd_preview.preview_frame as usize - 1)
                                            .unwrap(),
                                        &font_file.character_size,
                                    );
                                    changed |= true;
                                }
                            }

                            if ui.button("Reset").clicked() {
                                self.osd_options.position.x = 0;
                                changed |= true;
                            }
                        });
                        ui.end_row();

                        //

                        ui.label("Vertical position")
                            .on_hover_text(tooltip_text("Vertical position of the flight controller OSD (pixels from the top of the video).").small());
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(Slider::new(&mut self.osd_options.position.y, -200..=700).text("Pixels"))
                                .changed();

                            if ui.button("Center").clicked() {
                                if let (Some(video_info), Some(osd_file), Some(font_file)) =
                                    (&self.video_info, &self.osd_file, &self.font_file)
                                {
                                    self.osd_options.position.y = calculate_vertical_offset(
                                        video_info.height,
                                        osd_file
                                            .frames
                                            .get(self.osd_preview.preview_frame as usize - 1)
                                            .unwrap(),
                                        &font_file.character_size,
                                    );
                                    changed |= true
                                }
                            }

                            if ui.button("Reset").clicked() {
                                self.osd_options.position.y = 0;
                                changed |= true
                            }
                        });
                        ui.end_row();

                        ui.label("Mask")
                            .on_hover_text(tooltip_text("Click edit to select OSD elements on the preview that should not be rendered on the video. This can be useful to hide GPS coordinates, etc."));
                        ui.horizontal(|ui| {
                            let txt = if !self.osd_preview.mask_edit_mode_enabled || !self.all_files_loaded() {"Edit"} else {"Save"};
                            if ui.add_enabled(self.all_files_loaded(), Button::new(txt))
                                .on_disabled_hover_text(tooltip_text("First load the input files")).clicked() {
                                self.osd_preview.mask_edit_mode_enabled = !self.osd_preview.mask_edit_mode_enabled;
                            }
                            if ui.button("Reset").clicked() {
                                self.osd_options.reset_mask();
                                self.config_changed = Instant::now().into();
                                self.update_osd_preview(ctx);
                            }
                            let masked_positions = self.osd_options.masked_grid_positions.len();
                            ui.label(format!("{masked_positions} positions masked"));
                        });
                        ui.end_row();

                        ui.label("Adjust playback speed")
                            .on_hover_text(tooltip_text("Attempt to correct for wrong OSD timestamps in <=32.37.10 firmwares that causes video and OSD to get out of sync."));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(Checkbox::without_text(&mut self.osd_options.adjust_playback_speed))
                                .changed()
                        });
                    });
            });

        if changed {
            self.update_osd_preview(ctx);
            self.config_changed = Some(Instant::now());
        }
    }

    fn osd_preview(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        CollapsingHeader::new(RichText::new("Preview").heading())
            .default_open(true)
            .show_unindented(ui, |ui| {
                if let (Some(handle), Some(video_info)) = (&self.osd_preview.texture_handle, &self.video_info) {
                    let preview_width = ui.available_width();
                    let aspect_ratio = video_info.width as f32 / video_info.height as f32;
                    let preview_height = preview_width / aspect_ratio;
                    let image = Image::new(handle).fit_to_exact_size(Vec2::new(preview_width, preview_height));
                    let rect = ui.add(image.bg_fill(Color32::LIGHT_GRAY)).rect;

                    if self.osd_preview.mask_edit_mode_enabled {
                        self.draw_grid(ui, ctx, rect);
                    }

                    ui.horizontal(|ui| {
                        ui.label("Preview frame").on_hover_text(tooltip_text(
                            "The selected frame is also used for centering the OSD under OSD Options.",
                        ));
                        let preview_frame_slider = ui.add(
                            Slider::new(
                                &mut self.osd_preview.preview_frame,
                                1..=self.osd_file.as_ref().map(|f| f.frame_count).unwrap_or(1),
                            )
                            .smart_aim(false),
                        );
                        if preview_frame_slider.changed() {
                            self.update_osd_preview(ctx);
                        }
                    });
                }
            });
    }

    fn draw_grid(&mut self, ui: &mut Ui, ctx: &egui::Context, image_rect: Rect) {
        let video_width = self.video_info.as_ref().unwrap().width as f32;
        let video_height = self.video_info.as_ref().unwrap().height as f32;

        let top_left = image_rect.left_top();
        let preview_width = image_rect.width();
        let preview_height = image_rect.height();

        let grid_width = preview_width * 0.99375;
        let grid_height = preview_height;
        let cell_width = grid_width / 53.0;
        let cell_height = grid_height / 20.0;

        let painter = ui.painter_at(image_rect);

        let horizontal_offset = self.osd_options.position.x as f32 / video_width * preview_width;
        let vertical_offset = self.osd_options.position.y as f32 / video_height * preview_height;

        let response = ui
            .allocate_rect(image_rect, Sense::click())
            .on_hover_cursor(CursorIcon::Crosshair);

        for i in 0..53 {
            for j in 0..20 {
                let rect = Rect::from_min_size(
                    top_left
                        + vec2(i as f32 * cell_width, j as f32 * cell_height)
                        + vec2(horizontal_offset, vertical_offset),
                    vec2(cell_width, cell_height),
                );

                let grid_position = Coordinates::new(i, j);
                let masked = self.osd_options.get_mask(&grid_position);
                if masked {
                    painter.rect_filled(rect, 0.0, Color32::RED.gamma_multiply(0.5));
                }

                if let Some(hover_pos) = ctx.pointer_hover_pos() {
                    if rect.contains(hover_pos) {
                        painter.rect_filled(rect, 0.0, Color32::RED.gamma_multiply(0.2));
                    }
                }

                if response.clicked() {
                    if let Some(click_pos) = ctx.pointer_interact_pos() {
                        if rect.contains(click_pos) {
                            self.osd_options.toggle_mask(grid_position);
                            self.update_osd_preview(ctx);
                            self.config_changed = Instant::now().into();
                        }
                    }
                }
            }
        }

        let line_stroke = Stroke::new(1.0, Color32::GRAY.gamma_multiply(0.5));

        for i in 0..=53 {
            let x = top_left.x + i as f32 * cell_width + horizontal_offset;
            let y_min = image_rect.y_range().min + vertical_offset;
            let y_max = image_rect.y_range().max + vertical_offset;
            painter.vline(x, y_min..=y_max, line_stroke);
        }
        for i in 0..=20 {
            let x_min = image_rect.x_range().min + horizontal_offset;
            let x_max = image_rect.x_range().max + horizontal_offset;
            let y = top_left.y + i as f32 * cell_height + vertical_offset;
            painter.hline(x_min..=x_max, y, line_stroke);
        }
    }

    fn rendering_options(&mut self, ui: &mut Ui) {
        let mut changed = false;
        CollapsingHeader::new(RichText::new("Rendering Options").heading())
            .default_open(true)
            .show_unindented(ui, |ui| {
                let selectable_encoders = self
                    .encoders
                    .iter()
                    .filter(|e| self.render_settings.show_undetected_encoders || e.detected)
                    .collect::<Vec<_>>();

                Grid::new("render_options")
                    .min_col_width(self.ui_dimensions.options_column1_width)
                    .show(ui, |ui| {
                        ui.label("Output").on_hover_text(tooltip_text(
                            "Render a new video with the OSD on top, or only the OSD on a transparent background as a folder of PNG files (one per video frame) that can be imported as an image sequence in a video editor.",
                        ));
                        ui.horizontal(|ui| {
                            changed |= ui
                                .selectable_value(&mut self.render_settings.output_mode, OutputMode::Video, "Video")
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut self.render_settings.output_mode,
                                    OutputMode::PngSequence,
                                    "PNG sequence",
                                )
                                .changed();
                        });
                        ui.end_row();

                        if self.render_settings.output_mode == OutputMode::PngSequence {
                            ui.label("PNG frame rate").on_hover_text(tooltip_text(
                                "Write a PNG for every video frame, or only for every second frame to halve the number of files. Import the sequence in your video editor with half the frame rate of the video so it keeps the same duration.",
                            ));
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut self.render_settings.png_frame_rate,
                                        PngFrameRate::Full,
                                        "Same as video",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut self.render_settings.png_frame_rate,
                                        PngFrameRate::Half,
                                        "Half",
                                    )
                                    .changed();
                                if let Some(video_info) = &self.video_info {
                                    let fps = video_info.frame_rate / self.render_settings.png_frame_rate.step() as f32;
                                    let count = self.render_settings.png_frame_rate.frame_count(video_info.total_frames);
                                    ui.label(format!("{fps:.2} fps, about {count} images"));
                                }
                            });
                            ui.end_row();

                            ui.label("Output folder").on_hover_text(tooltip_text(
                                "A folder named after the video with the suffix _osd is created here and filled with the PNG files. By default it is created next to the input video.",
                            ));
                            ui.horizontal(|ui| {
                                let location = match (&self.render_settings.png_output_parent, &self.video_file) {
                                    (Some(parent), _) => Some(parent.clone()),
                                    (None, Some(video)) => video.parent().map(|p| p.to_path_buf()),
                                    (None, None) => None,
                                };
                                let folder_text = match (&location, &self.video_file) {
                                    (Some(_), Some(video)) => {
                                        png_sequence_dir(video, self.render_settings.png_output_parent.as_deref())
                                            .to_string_lossy()
                                            .into_owned()
                                    }
                                    (Some(parent), None) => parent.to_string_lossy().into_owned(),
                                    (None, _) => "Next to the input video".to_string(),
                                };
                                ui.label(folder_text);
                                if ui.button("Choose...").clicked() {
                                    let mut dialog = rfd::FileDialog::new();
                                    if let Some(location) = &location {
                                        dialog = dialog.set_directory(location);
                                    }
                                    if let Some(folder) = dialog.pick_folder() {
                                        self.render_settings.png_output_parent = Some(folder);
                                        changed |= true;
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        self.render_settings.png_output_parent.is_some(),
                                        Button::new("Reset"),
                                    )
                                    .clicked()
                                {
                                    self.render_settings.png_output_parent = None;
                                    changed |= true;
                                }
                            });
                            ui.end_row();
                        } else {
                        ui.label("Encoder")
                            .on_hover_text(tooltip_text("Encoder used for rendering. In some cases not all available encoders are detected. Check the box to also show these."));
                        ui.horizontal(|ui| {
                            let selection = egui::ComboBox::from_id_source("encoder").width(350.0).show_index(
                                ui,
                                &mut self.render_settings.selected_encoder_idx,
                                selectable_encoders.len(),
                                |i| {
                                    selectable_encoders
                                        .get(i)
                                        .map(|e| e.to_string())
                                        .unwrap_or("None".to_string())
                                },
                            );
                            if selection.changed() {
                                // This is a little hacky but it's nice to have a single struct that keeps track of all render settings
                                self.render_settings.encoder =
                                    (*selectable_encoders.get(self.render_settings.selected_encoder_idx).unwrap()).clone();
                                changed |= true;
                            }

                            if ui
                                .add(Checkbox::without_text(&mut self.render_settings.show_undetected_encoders))
                                .changed() {
                                    self.render_settings.selected_encoder_idx = 0;
                                    self.render_settings.encoder =
                                        (*selectable_encoders.first().unwrap()).clone();
                                    changed |= true;
                            }
                        });
                        ui.end_row();

                        ui.label("Encoding bitrate").on_hover_text(tooltip_text("Target bitrate of the rendered video."));
                        changed |= ui.add(Slider::new(&mut self.render_settings.bitrate_mbps, 0..=160).text("Mbps")).changed();
                        ui.end_row();

                        ui.label("Upscale to 1440p").on_hover_text(tooltip_text("Upscale the output video to 1440p to get better quality after uplaoding to YouTube."));
                        changed |= ui.add(Checkbox::without_text(&mut self.render_settings.upscale)).changed();
                        ui.end_row();

                        ui.label("Chroma key").on_hover_text(tooltip_text("Render the video with a chroma key instead of the input video so the OSD can be overlay in video editing software."));
                        ui.horizontal(|ui| {
                            changed |= ui.add(Checkbox::without_text(&mut self.render_settings.use_chroma_key)).changed();
                            changed |= ui.color_edit_button_rgb(&mut self.render_settings.chroma_key).changed();
                        });
                        ui.end_row();
                        }
                    });
            });

        if changed {
            self.config_changed = Some(Instant::now());
        }
    }
}
