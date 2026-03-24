use eframe::egui;

use crate::parser::{self, Replay};

pub struct App {
    replay: Option<Replay>,
    error: Option<String>,
    dropped_file: Option<Vec<u8>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            replay: None,
            error: None,
            dropped_file: None,
        }
    }
}

impl App {
    fn load_replay(&mut self, data: Vec<u8>) {
        match parser::parse_replay(&data) {
            Ok(replay) => {
                self.replay = Some(replay);
                self.error = None;
            }
            Err(e) => {
                self.replay = None;
                self.error = Some(e);
            }
        }
    }

    fn render_welcome(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading("PathToBonjwa");
            ui.add_space(8.0);
            ui.label("Brood War Replay Analyzer");
            ui.add_space(40.0);
            ui.label("Drop a .rep file here or click Open to load a replay");
            ui.add_space(16.0);
        });
    }

    fn render_summary(&self, ui: &mut egui::Ui, replay: &Replay) {
        ui.add_space(8.0);

        // Matchup header
        ui.horizontal(|ui| {
            ui.heading(&replay.matchup);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", replay.game_type))
                        .color(egui::Color32::GRAY),
                );
            });
        });

        ui.separator();

        // Map and game info
        egui::Grid::new("game_info")
            .num_columns(2)
            .spacing([20.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Map").strong());
                ui.label(format!(
                    "{} ({}x{})",
                    replay.map_name, replay.map_width, replay.map_height
                ));
                ui.end_row();

                ui.label(egui::RichText::new("Duration").strong());
                let mins = (replay.duration_secs / 60.0) as u32;
                let secs = (replay.duration_secs % 60.0) as u32;
                ui.label(format!("{}:{:02} ({} frames)", mins, secs, replay.frames));
                ui.end_row();

                ui.label(egui::RichText::new("Date").strong());
                let dt = chrono::DateTime::from_timestamp(replay.timestamp, 0);
                let date_str = match dt {
                    Some(d) => d.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    None => format!("timestamp: {}", replay.timestamp),
                };
                ui.label(date_str);
                ui.end_row();

                ui.label(egui::RichText::new("Engine").strong());
                ui.label(format!("{}", replay.engine));
                ui.end_row();

                ui.label(egui::RichText::new("Speed").strong());
                ui.label(format!("{}", replay.game_speed));
                ui.end_row();

                if !replay.host_name.is_empty() {
                    ui.label(egui::RichText::new("Host").strong());
                    ui.label(&replay.host_name);
                    ui.end_row();
                }

                if !replay.title.is_empty() {
                    ui.label(egui::RichText::new("Title").strong());
                    ui.label(&replay.title);
                    ui.end_row();
                }
            });

        ui.add_space(16.0);
        ui.heading("Players");
        ui.separator();

        egui::Grid::new("players")
            .num_columns(4)
            .spacing([20.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Name").strong());
                ui.label(egui::RichText::new("Race").strong());
                ui.label(egui::RichText::new("Type").strong());
                ui.label(egui::RichText::new("Team").strong());
                ui.end_row();

                for player in &replay.players {
                    let color = player.color.to_egui();
                    ui.label(egui::RichText::new(&player.name).color(color));
                    ui.label(format!("{}", player.race));
                    ui.label(format!("{}", player.player_type));
                    ui.label(format!("{}", player.team));
                    ui.end_row();
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle dropped files
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(bytes) = &file.bytes {
                    self.dropped_file = Some(bytes.to_vec());
                } else if let Some(path) = &file.path {
                    if let Ok(data) = std::fs::read(path) {
                        self.dropped_file = Some(data);
                    }
                }
            }
        });

        if let Some(data) = self.dropped_file.take() {
            self.load_replay(data);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("PathToBonjwa")
                        .strong()
                        .size(16.0),
                );
                ui.separator();
                if ui.button("Open Replay").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("BW Replay", &["rep"])
                        .pick_file()
                    {
                        match std::fs::read(&path) {
                            Ok(data) => self.load_replay(data),
                            Err(e) => {
                                self.error = Some(format!("Failed to read file: {}", e));
                                self.replay = None;
                            }
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref error) = self.error {
                ui.add_space(20.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("Error: {}", error),
                );
                ui.add_space(20.0);
                self.render_welcome(ui);
            } else if let Some(replay) = self.replay.clone() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_summary(ui, &replay);
                });
            } else {
                self.render_welcome(ui);
            }
        });

        // Show drag-and-drop overlay
        preview_files_being_dropped(ctx);
    }
}

fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::{Area, Color32, Id, LayerId, Order};

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(180));

        Area::new(Id::new("drop_text"))
            .fixed_pos(screen_rect.center())
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Drop .rep file here")
                        .heading()
                        .color(Color32::WHITE),
                );
            });
    }
}
