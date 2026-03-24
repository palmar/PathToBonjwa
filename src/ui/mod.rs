use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::analytics::{self, ApmData, BuildOrderEntry, HotkeyStats, UnitCount};
use crate::parser::{self, Replay};

// ─── App state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Summary,
    Stats,
    Charts,
}

pub struct App {
    replay: Option<Replay>,
    error: Option<String>,
    dropped_file: Option<Vec<u8>>,
    active_tab: Tab,
    // Cached analytics (computed once per replay load)
    cached: Option<CachedAnalytics>,
}

struct CachedAnalytics {
    apm_data: Vec<(u8, String, ApmData)>, // (player_id, name, data)
    build_orders: Vec<(u8, String, Vec<BuildOrderEntry>)>,
    unit_counts: Vec<(u8, String, Vec<UnitCount>)>,
    hotkey_stats: Vec<(u8, String, HotkeyStats)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            replay: None,
            error: None,
            dropped_file: None,
            active_tab: Tab::Summary,
            cached: None,
        }
    }
}

impl App {
    fn load_replay(&mut self, data: Vec<u8>) {
        match parser::parse_replay(&data) {
            Ok(replay) => {
                self.cached = Some(Self::compute_analytics(&replay));
                self.replay = Some(replay);
                self.error = None;
            }
            Err(e) => {
                self.replay = None;
                self.cached = None;
                self.error = Some(e);
            }
        }
    }

    fn compute_analytics(replay: &Replay) -> CachedAnalytics {
        let mut apm_data = Vec::new();
        let mut build_orders = Vec::new();
        let mut unit_counts = Vec::new();
        let mut hotkey_stats = Vec::new();

        for player in &replay.players {
            let pid = player.player_id;
            let name = player.name.clone();

            let apm = analytics::compute_apm(&replay.commands, pid, replay.frames);
            apm_data.push((pid, name.clone(), apm));

            let bo = analytics::extract_build_order(&replay.commands, pid);
            build_orders.push((pid, name.clone(), bo));

            let uc = analytics::compute_unit_counts(&replay.commands, pid);
            unit_counts.push((pid, name.clone(), uc));

            let hs = analytics::compute_hotkey_stats(&replay.commands, pid);
            hotkey_stats.push((pid, name, hs));
        }

        CachedAnalytics {
            apm_data,
            build_orders,
            unit_counts,
            hotkey_stats,
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
                    egui::RichText::new(format!("{}", replay.game_type)).color(egui::Color32::GRAY),
                );
            });
        });
        ui.separator();

        // APM summary
        if let Some(ref cached) = self.cached {
            for (_, name, apm) in &cached.apm_data {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(name).strong());
                    ui.label(format!(
                        "APM: {:.0}  |  EAPM: {:.0}",
                        apm.avg_apm, apm.avg_eapm
                    ));
                });
            }
            ui.separator();
        }

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

                ui.label(egui::RichText::new("Commands").strong());
                ui.label(format!("{} parsed", replay.commands.len()));
                ui.end_row();
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

    fn render_stats(&self, ui: &mut egui::Ui, replay: &Replay) {
        let cached = match &self.cached {
            Some(c) => c,
            None => return,
        };

        ui.add_space(8.0);

        // ─── Build orders ────────────────────────────────────────────────
        ui.heading("Build Orders");
        ui.separator();

        for (pid, name, entries) in &cached.build_orders {
            let player = replay.players.iter().find(|p| p.player_id == *pid);
            let color = player
                .map(|p| p.color.to_egui())
                .unwrap_or(egui::Color32::WHITE);

            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} ({} actions)", name, entries.len()))
                    .color(color)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new(format!("bo_{}", pid))
                    .num_columns(2)
                    .spacing([20.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Time").strong());
                        ui.label(egui::RichText::new("Unit / Building").strong());
                        ui.end_row();

                        for entry in entries.iter().take(50) {
                            ui.label(
                                egui::RichText::new(&entry.time_str)
                                    .color(egui::Color32::GRAY)
                                    .monospace(),
                            );
                            let style = if parser::is_building(entry.unit_id) {
                                egui::RichText::new(&entry.unit_name)
                                    .color(egui::Color32::LIGHT_BLUE)
                            } else {
                                egui::RichText::new(&entry.unit_name)
                            };
                            ui.label(style);
                            ui.end_row();
                        }
                        if entries.len() > 50 {
                            ui.label("");
                            ui.label(
                                egui::RichText::new(format!("... and {} more", entries.len() - 50))
                                    .color(egui::Color32::GRAY),
                            );
                            ui.end_row();
                        }
                    });
            });
            ui.add_space(8.0);
        }

        // ─── Unit production counts ──────────────────────────────────────
        ui.add_space(8.0);
        ui.heading("Unit Production");
        ui.separator();

        for (pid, name, counts) in &cached.unit_counts {
            let player = replay.players.iter().find(|p| p.player_id == *pid);
            let color = player
                .map(|p| p.color.to_egui())
                .unwrap_or(egui::Color32::WHITE);

            egui::CollapsingHeader::new(egui::RichText::new(name).color(color).strong())
                .default_open(true)
                .show(ui, |ui| {
                    let buildings: Vec<&UnitCount> =
                        counts.iter().filter(|c| c.is_building).collect();
                    let units: Vec<&UnitCount> = counts.iter().filter(|c| !c.is_building).collect();

                    if !buildings.is_empty() {
                        ui.label(
                            egui::RichText::new("Buildings")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        egui::Grid::new(format!("bldg_{}", pid))
                            .num_columns(2)
                            .spacing([20.0, 2.0])
                            .show(ui, |ui| {
                                for b in &buildings {
                                    ui.label(
                                        egui::RichText::new(&b.unit_name)
                                            .color(egui::Color32::LIGHT_BLUE),
                                    );
                                    ui.label(format!("x{}", b.count));
                                    ui.end_row();
                                }
                            });
                        ui.add_space(4.0);
                    }

                    if !units.is_empty() {
                        ui.label(
                            egui::RichText::new("Units")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        egui::Grid::new(format!("units_{}", pid))
                            .num_columns(2)
                            .spacing([20.0, 2.0])
                            .show(ui, |ui| {
                                for u in &units {
                                    ui.label(&u.unit_name);
                                    ui.label(format!("x{}", u.count));
                                    ui.end_row();
                                }
                            });
                    }
                });
            ui.add_space(8.0);
        }

        // ─── Hotkey stats ────────────────────────────────────────────────
        ui.add_space(8.0);
        ui.heading("Hotkey Usage");
        ui.separator();

        for (pid, name, stats) in &cached.hotkey_stats {
            let player = replay.players.iter().find(|p| p.player_id == *pid);
            let color = player
                .map(|p| p.color.to_egui())
                .unwrap_or(egui::Color32::WHITE);

            let total: u32 = stats.groups.iter().map(|g| g.total()).sum();
            if total == 0 {
                continue;
            }

            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} ({} total)", name, total))
                    .color(color)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new(format!("hk_{}", pid))
                    .num_columns(5)
                    .spacing([12.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Group").strong());
                        ui.label(egui::RichText::new("Assign").strong());
                        ui.label(egui::RichText::new("Select").strong());
                        ui.label(egui::RichText::new("Add").strong());
                        ui.label(egui::RichText::new("Total").strong());
                        ui.end_row();

                        for (i, group) in stats.groups.iter().enumerate() {
                            if group.total() == 0 {
                                continue;
                            }
                            ui.label(format!("{}", i));
                            ui.label(format!("{}", group.assigns));
                            ui.label(format!("{}", group.selects));
                            ui.label(format!("{}", group.adds));
                            ui.label(egui::RichText::new(format!("{}", group.total())).strong());
                            ui.end_row();
                        }
                    });
            });
            ui.add_space(8.0);
        }
    }

    fn render_charts(&self, ui: &mut egui::Ui, replay: &Replay) {
        let cached = match &self.cached {
            Some(c) => c,
            None => return,
        };

        ui.add_space(8.0);
        ui.heading("APM Over Time");
        ui.separator();
        ui.add_space(4.0);

        let plot_height = 250.0;

        // APM chart
        Plot::new("apm_chart")
            .height(plot_height)
            .x_axis_label("Minute")
            .y_axis_label("Actions per Minute")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (pid, name, apm) in &cached.apm_data {
                    let player = replay.players.iter().find(|p| p.player_id == *pid);
                    let color = player
                        .map(|p| p.color.to_egui())
                        .unwrap_or(egui::Color32::WHITE);

                    let points: PlotPoints =
                        apm.apm_per_minute.iter().map(|&(x, y)| [x, y]).collect();
                    let line = Line::new(points)
                        .name(format!("{} APM", name))
                        .color(color)
                        .width(2.0);
                    plot_ui.line(line);
                }
            });

        ui.add_space(16.0);
        ui.heading("EAPM Over Time");
        ui.separator();
        ui.add_space(4.0);

        Plot::new("eapm_chart")
            .height(plot_height)
            .x_axis_label("Minute")
            .y_axis_label("Effective APM")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (pid, name, apm) in &cached.apm_data {
                    let player = replay.players.iter().find(|p| p.player_id == *pid);
                    let color = player
                        .map(|p| p.color.to_egui())
                        .unwrap_or(egui::Color32::WHITE);

                    let points: PlotPoints =
                        apm.eapm_per_minute.iter().map(|&(x, y)| [x, y]).collect();
                    let line = Line::new(points)
                        .name(format!("{} EAPM", name))
                        .color(color)
                        .width(2.0);
                    plot_ui.line(line);
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
                ui.label(egui::RichText::new("PathToBonjwa").strong().size(16.0));
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
                                self.cached = None;
                            }
                        }
                    }
                }

                // Tab buttons (only show when a replay is loaded)
                if self.replay.is_some() {
                    ui.separator();
                    if ui
                        .selectable_label(self.active_tab == Tab::Summary, "Summary")
                        .clicked()
                    {
                        self.active_tab = Tab::Summary;
                    }
                    if ui
                        .selectable_label(self.active_tab == Tab::Stats, "Stats")
                        .clicked()
                    {
                        self.active_tab = Tab::Stats;
                    }
                    if ui
                        .selectable_label(self.active_tab == Tab::Charts, "Charts")
                        .clicked()
                    {
                        self.active_tab = Tab::Charts;
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
                egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                    Tab::Summary => self.render_summary(ui, &replay),
                    Tab::Stats => self.render_stats(ui, &replay),
                    Tab::Charts => self.render_charts(ui, &replay),
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
