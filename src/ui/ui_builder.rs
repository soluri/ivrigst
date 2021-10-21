use crate::{
    model::{Attributes, DistanceShadingChannel},
    resources::Resources,
    ui::sdl2_egui_translation::egui_to_sdl2_cursor,
};
use anyhow::{anyhow, Result};
use nalgebra as na;

use super::UIRenderer;

pub struct UI {
    pub renderer: UIRenderer,
    preset: Preset,
    model_files: Vec<String>,
}

pub struct UiActions {
    pub show_debug: bool,
    pub file_to_load: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    Aerial,
    ChromaDepth,
    Plain,
}

impl Preset {
    pub fn description(&self) -> &'static str {
        match self {
            Preset::Plain => "Plain",
            Preset::Aerial => "Aerial",
            Preset::ChromaDepth => "Colored depth",
        }
    }
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(self.description())
    }
}

impl UI {
    pub fn new(res: &Resources) -> Result<Self> {
        let renderer = UIRenderer::new(res)?;
        let preset = Preset::Plain;
        let model_files = res.list_models();
        Ok(Self {
            renderer,
            preset,
            model_files,
        })
    }

    pub fn build_ui(
        &mut self,
        ctx: &egui::CtxRef,
        model: &mut Option<crate::Model>,
        ui_actions: &mut UiActions,
    ) {
        egui::Window::new("Settings")
            .auto_sized()
            .collapsible(true)
            .show(ctx, |ui| {
                let mut selected_file = String::new();
                if ui_actions.file_to_load.is_empty() {
                    selected_file.push_str("No file loaded");
                } else {
                    selected_file.push_str("Current file: '");
                    selected_file.push_str(&ui_actions.file_to_load);
                    selected_file.push('\'');
                }
                egui::ComboBox::from_id_source("model")
                    .selected_text(selected_file)
                    .show_ui(ui, |ui| {
                        for file in &self.model_files {
                            ui.selectable_value(&mut ui_actions.file_to_load, file.clone(), file);
                        }
                    });
                ui.end_row();

                if let Some(model) = model {
                    let mut attr = model.get_attributes().clone();

                    ui.label("Choose visualization preset:");

                    ui.horizontal(|ui| {
                        if ui.button(Preset::Plain.description()).clicked() {
                            self.preset = Preset::Plain;
                            attr = self.apply_preset(model);
                        }
                        if ui.button(Preset::Aerial.description()).clicked() {
                            self.preset = Preset::Aerial;
                            attr = self.apply_preset(model);
                        }
                        if ui.button(Preset::ChromaDepth.description()).clicked() {
                            self.preset = Preset::ChromaDepth;
                            attr = self.apply_preset(model);
                        }
                    });

                    ui.collapsing("Advanced", |ui| {
                        egui::Grid::new("settings_grid")
                            .striped(true)
                            .spacing([40.0, 4.0])
                            .show(ui, |ui| {
                                // Colour widget.
                                ui.label("Model base colour");
                                let mut color = [attr.color[0], attr.color[1], attr.color[2]];
                                ui.color_edit_button_rgb(&mut color);
                                attr.color = na::Vector3::from(color);
                                ui.end_row();

                                ui.label("Model colouring mix");
                                ui.add(egui::Slider::new(&mut attr.vertex_color_mix, 0.0..=1.0));
                                ui.end_row();

                                // Toon shading enable/disable
                                ui.label("Toon shading factor");
                                ui.add(egui::Slider::new(&mut attr.toon_factor, 0.0..=1.0));
                                ui.end_row();

                                // Distance shading parameters widget.
                                use crate::model::DistanceShadingChannel as DSC;
                                ui.label("Distance shading channel");
                                egui::ComboBox::from_id_source("distance_shading_channel")
                                    .selected_text(attr.distance_shading_channel.to_string())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut attr.distance_shading_channel,
                                            DSC::Hue,
                                            DSC::Hue.to_string(),
                                        );
                                        ui.selectable_value(
                                            &mut attr.distance_shading_channel,
                                            DSC::Saturation,
                                            DSC::Saturation.to_string(),
                                        );
                                        ui.selectable_value(
                                            &mut attr.distance_shading_channel,
                                            DSC::Value,
                                            DSC::Value.to_string(),
                                        );
                                        ui.selectable_value(
                                            &mut attr.distance_shading_channel,
                                            DSC::None,
                                            DSC::None.to_string(),
                                        );
                                    });
                                ui.end_row();

                                ui.label("Distance shading power");
                                ui.add(egui::Slider::new(
                                    &mut attr.distance_shading_power,
                                    0.0..=1.0,
                                ));

                                ui.end_row();

                                ui.label("Display shader buffers");
                                ui.checkbox(&mut ui_actions.show_debug, "");
                                ui.end_row();

                                ui.label("Use hatching instead of shadows");
                                ui.checkbox(&mut attr.replace_shadows_with_hatching, "");
                                ui.end_row();
                            });

                        ui.collapsing("Hatching settings", |ui| {
                            egui::Grid::new("hatching_settings_grid")
                                .striped(true)
                                .spacing([40.0, 4.0])
                                .show(ui, |ui| {
                                    ui.set_enabled(attr.replace_shadows_with_hatching);
                                    ui.label("Hatching depth");
                                    ui.add(egui::Slider::new(&mut attr.hatching_depth, 0.0..=3.0));
                                    ui.end_row();

                                    ui.label("Hatching steps");
                                    ui.add(egui::Slider::new(&mut attr.hatching_steps, 1..=250));
                                    ui.end_row();

                                    ui.label("Hatching frequency");
                                    ui.add(egui::Slider::new(&mut attr.hatching_frequency, 1..=15));
                                    ui.end_row();

                                    ui.label("Hatching intensity");
                                    ui.add(egui::Slider::new(
                                        &mut attr.hatching_intensity,
                                        0.0..=1.0,
                                    ));
                                    ui.end_row();
                                })
                        });

                        ui.collapsing("Shadow settings", |ui| {
                            egui::Grid::new("shadow_settings_grid")
                                .striped(true)
                                .spacing([40.0, 4.0])
                                .show(ui, |ui| {
                                    ui.set_enabled(!attr.replace_shadows_with_hatching);
                                    ui.label("Shadow intensity");
                                    ui.add(egui::Slider::new(
                                        &mut attr.shadow_intensity,
                                        0.0..=1.0,
                                    ));
                                    ui.end_row();

                                    ui.label("Light follows camera");
                                    ui.checkbox(&mut attr.shadows_follow, "");
                                    ui.end_row();

                                    ui.label("Light X");
                                    ui.scope(|ui| {
                                        ui.set_enabled(!attr.shadows_follow);
                                        ui.add(egui::Slider::new(
                                            &mut attr.light_position[0],
                                            -1.0..=1.0,
                                        ))
                                        .on_disabled_hover_text("Disabled while following camera.");
                                    });
                                    ui.end_row();
                                    ui.label("Light Y");
                                    ui.scope(|ui| {
                                        ui.set_enabled(!attr.shadows_follow);
                                        ui.add(egui::Slider::new(
                                            &mut attr.light_position[1],
                                            -1.0..=1.0,
                                        ))
                                        .on_disabled_hover_text("Disabled while following camera.");
                                    });
                                    ui.end_row();
                                    ui.label("Light Z");
                                    ui.scope(|ui| {
                                        ui.set_enabled(!attr.shadows_follow);
                                        ui.add(egui::Slider::new(
                                            &mut attr.light_position[2],
                                            -1.0..=1.0,
                                        ))
                                        .on_disabled_hover_text("Disabled while following camera.");
                                    });
                                    ui.end_row();
                                    ui.label("Light orbit distance");
                                    ui.add(egui::Slider::new(
                                        &mut attr.shadows_orbit_radius,
                                        0.0..=100.0,
                                    ));
                                    ui.end_row();
                                })
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Read more at:");
                        ui.add(egui::Hyperlink::new("https://github.com/stisol/rmedvis"));
                    });
                    model.set_attributes(attr);
                }
            });
    }

    pub fn apply_preset(&self, model: &mut crate::Model) -> Attributes {
        let mut preset = model.get_attributes().clone();
        match self.preset {
            Preset::Plain => {
                preset.toon_factor = 0.0;
                preset.vertex_color_mix = 1.0;
                preset.distance_shading_channel = DistanceShadingChannel::None;

                preset.replace_shadows_with_hatching = true;
                preset.hatching_depth = 0.75;
                preset.hatching_steps = 150;
                preset.hatching_frequency = 4;
                preset.hatching_intensity = 0.75;
            }
            Preset::Aerial => {
                preset.toon_factor = 0.0;
                preset.vertex_color_mix = 1.0;
                preset.distance_shading_channel = DistanceShadingChannel::Saturation;
                preset.distance_shading_power = 0.6;

                preset.replace_shadows_with_hatching = true;
                preset.hatching_depth = 0.75;
                preset.hatching_steps = 150;
                preset.hatching_frequency = 4;
                preset.hatching_intensity = 0.75;
            }
            Preset::ChromaDepth => {
                preset.toon_factor = 0.0;
                preset.distance_shading_channel = DistanceShadingChannel::Hue;
                preset.distance_shading_power = 0.6;

                preset.replace_shadows_with_hatching = true;
                preset.hatching_depth = 0.75;
                preset.hatching_steps = 150;
                preset.hatching_frequency = 4;
                preset.hatching_intensity = 0.75;
            }
        };
        model.set_attributes(preset.clone());
        preset
    }

    pub fn handle_output(&self, output: egui::Output) -> Result<sdl2::mouse::Cursor> {
        let system_cursor = egui_to_sdl2_cursor(output.cursor_icon);
        let cursor = sdl2::mouse::Cursor::from_system(system_cursor).map_err(|e| anyhow!(e))?;

        if !output.copied_text.is_empty() {
            use clipboard::{ClipboardContext, ClipboardProvider};
            let mut ctx: ClipboardContext =
                ClipboardProvider::new().map_err(|_| anyhow!("Could not open clipboard."))?;
            ctx.set_contents(output.copied_text)
                .map_err(|_| anyhow!("Could not set clipboard text."))?;
        }

        if let Some(url) = output.open_url {
            if let Err(e) = webbrowser::open(&url.url) {
                eprintln!("Error opening link: {}", e);
            }
        }

        Ok(cursor)
    }
}