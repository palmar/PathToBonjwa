use std::time::Instant;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::analytics::{
    self, ApmData, BuildOrderEntry, HotkeyStats, IdleAnalysis, ResourceEstimate, SupplyCurve,
    UnitCount, UnitProductionSpan,
};
use crate::parser::{self, Replay};

// ─── App state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Summary,
    Stats,
    Charts,
    Analytics,
    Batch,
    Logs,
}

#[derive(Clone)]
struct LogEntry {
    elapsed_secs: f64,
    level: LogLevel,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum LogLevel {
    Info,
    Warn,
    Error,
}

pub struct App {
    replay: Option<Replay>,
    error: Option<String>,
    dropped_file: Option<Vec<u8>>,
    active_tab: Tab,
    cached: Option<CachedAnalytics>,
    // Batch mode: multiple loaded replays
    batch_replays: Vec<BatchEntry>,
    batch_selected: Option<usize>,
    // Client-side logging
    log_entries: Vec<LogEntry>,
    log_start: Instant,
    log_auto_scroll: bool,
}

struct BatchEntry {
    filename: String,
    replay: Replay,
    cached: CachedAnalytics,
}

struct CachedAnalytics {
    apm_data: Vec<(u8, String, ApmData)>,
    build_orders: Vec<(u8, String, Vec<BuildOrderEntry>)>,
    unit_counts: Vec<(u8, String, Vec<UnitCount>)>,
    hotkey_stats: Vec<(u8, String, HotkeyStats)>,
    // Phase 3
    supply_curves: Vec<(u8, String, SupplyCurve)>,
    production_spans: Vec<(u8, String, Vec<UnitProductionSpan>)>,
    resource_estimates: Vec<(u8, String, ResourceEstimate)>,
    idle_analyses: Vec<(u8, String, IdleAnalysis)>,
}

impl Default for App {
    fn default() -> Self {
        let start = Instant::now();
        let mut app = Self {
            replay: None,
            error: None,
            dropped_file: None,
            active_tab: Tab::Summary,
            cached: None,
            batch_replays: Vec::new(),
            batch_selected: None,
            log_entries: Vec::new(),
            log_start: start,
            log_auto_scroll: true,
        };
        app.log(LogLevel::Info, "PathToBonjwa started");
        app
    }
}

impl App {
    fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_entries.push(LogEntry {
            elapsed_secs: self.log_start.elapsed().as_secs_f64(),
            level,
            message: message.into(),
        });
    }
}

impl App {
    fn load_replay(&mut self, data: Vec<u8>) {
        self.log(LogLevel::Info, format!("Parsing replay ({} bytes)", data.len()));
        match parser::parse_replay(&data) {
            Ok(replay) => {
                self.log(
                    LogLevel::Info,
                    format!(
                        "Replay parsed: {} — {} players, {} frames, {} commands",
                        replay.matchup,
                        replay.players.len(),
                        replay.frames,
                        replay.commands.len(),
                    ),
                );
                self.log(LogLevel::Info, "Computing analytics...");
                self.cached = Some(Self::compute_analytics(&replay));
                self.log(LogLevel::Info, "Analytics computed");
                self.replay = Some(replay);
                self.error = None;
            }
            Err(e) => {
                self.log(LogLevel::Error, format!("Parse error: {}", e));
                self.replay = None;
                self.cached = None;
                self.error = Some(e);
            }
        }
    }

    fn load_replay_batch(&mut self, filename: String, data: Vec<u8>) {
        match parser::parse_replay(&data) {
            Ok(replay) => {
                let cached = Self::compute_analytics(&replay);
                self.log(LogLevel::Info, format!("Batch loaded: {}", filename));
                self.batch_replays.push(BatchEntry {
                    filename,
                    replay,
                    cached,
                });
            }
            Err(e) => {
                self.log(LogLevel::Warn, format!("Batch skip {}: {}", filename, e));
            }
        }
    }

    fn compute_analytics(replay: &Replay) -> CachedAnalytics {
        let mut apm_data = Vec::new();
        let mut build_orders = Vec::new();
        let mut unit_counts = Vec::new();
        let mut hotkey_stats = Vec::new();
        let mut supply_curves = Vec::new();
        let mut production_spans = Vec::new();
        let mut resource_estimates = Vec::new();
        let mut idle_analyses = Vec::new();

        for player in &replay.players {
            let pid = player.player_id;
            let name = player.name.clone();

            apm_data.push((
                pid,
                name.clone(),
                analytics::compute_apm(&replay.commands, pid, replay.frames),
            ));
            build_orders.push((
                pid,
                name.clone(),
                analytics::extract_build_order(&replay.commands, pid),
            ));
            unit_counts.push((
                pid,
                name.clone(),
                analytics::compute_unit_counts(&replay.commands, pid),
            ));
            hotkey_stats.push((
                pid,
                name.clone(),
                analytics::compute_hotkey_stats(&replay.commands, pid),
            ));
            supply_curves.push((
                pid,
                name.clone(),
                analytics::compute_supply_curve(&replay.commands, pid, &player.race, replay.frames),
            ));
            production_spans.push((
                pid,
                name.clone(),
                analytics::compute_production_spans(&replay.commands, pid),
            ));
            resource_estimates.push((
                pid,
                name.clone(),
                analytics::compute_resource_estimate(&replay.commands, pid),
            ));
            idle_analyses.push((
                pid,
                name.clone(),
                analytics::compute_idle_gaps(&replay.commands, pid, replay.frames, 5.0),
            ));
        }

        CachedAnalytics {
            apm_data,
            build_orders,
            unit_counts,
            hotkey_stats,
            supply_curves,
            production_spans,
            resource_estimates,
            idle_analyses,
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

        // ─── Supply curve chart ──────────────────────────────────────────
        ui.add_space(16.0);
        ui.heading("Supply Over Time");
        ui.separator();
        ui.add_space(4.0);

        Plot::new("supply_chart")
            .height(plot_height)
            .x_axis_label("Seconds")
            .y_axis_label("Supply")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (pid, name, curve) in &cached.supply_curves {
                    let player = replay.players.iter().find(|p| p.player_id == *pid);
                    let color = player
                        .map(|p| p.color.to_egui())
                        .unwrap_or(egui::Color32::WHITE);

                    // Supply used line
                    let used_points: PlotPoints =
                        curve.points.iter().map(|&(t, used, _)| [t, used]).collect();
                    plot_ui.line(
                        Line::new(used_points)
                            .name(format!("{} Used", name))
                            .color(color)
                            .width(2.0),
                    );

                    // Supply max line (dashed effect via lighter color)
                    let max_points: PlotPoints =
                        curve.points.iter().map(|&(t, _, max)| [t, max]).collect();
                    let max_color =
                        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120);
                    plot_ui.line(
                        Line::new(max_points)
                            .name(format!("{} Max", name))
                            .color(max_color)
                            .width(1.5),
                    );
                }
            });

        // ─── Resource spending chart ─────────────────────────────────────
        ui.add_space(16.0);
        ui.heading("Cumulative Resource Spending");
        ui.separator();
        ui.add_space(4.0);

        Plot::new("resource_chart")
            .height(plot_height)
            .x_axis_label("Seconds")
            .y_axis_label("Resources Spent")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (pid, name, res) in &cached.resource_estimates {
                    let player = replay.players.iter().find(|p| p.player_id == *pid);
                    let color = player
                        .map(|p| p.color.to_egui())
                        .unwrap_or(egui::Color32::WHITE);

                    // Minerals line
                    let min_points: PlotPoints = res
                        .spending_curve
                        .iter()
                        .map(|&(t, m, _)| [t, m as f64])
                        .collect();
                    plot_ui.line(
                        Line::new(min_points)
                            .name(format!("{} Minerals", name))
                            .color(color)
                            .width(2.0),
                    );

                    // Gas line (lighter)
                    let gas_points: PlotPoints = res
                        .spending_curve
                        .iter()
                        .map(|&(t, _, g)| [t, g as f64])
                        .collect();
                    let gas_color =
                        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 140);
                    plot_ui.line(
                        Line::new(gas_points)
                            .name(format!("{} Gas", name))
                            .color(gas_color)
                            .width(1.5),
                    );
                }
            });
    }

    fn render_analytics(&self, ui: &mut egui::Ui, replay: &Replay) {
        let cached = match &self.cached {
            Some(c) => c,
            None => return,
        };

        ui.add_space(8.0);

        // ─── Resource estimates ──────────────────────────────────────────
        ui.heading("Resource Estimates");
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("resource_summary")
            .num_columns(3)
            .spacing([20.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Player").strong());
                ui.label(egui::RichText::new("Minerals Spent").strong());
                ui.label(egui::RichText::new("Gas Spent").strong());
                ui.end_row();

                for (pid, name, res) in &cached.resource_estimates {
                    let player = replay.players.iter().find(|p| p.player_id == *pid);
                    let color = player
                        .map(|p| p.color.to_egui())
                        .unwrap_or(egui::Color32::WHITE);
                    ui.label(egui::RichText::new(name).color(color).strong());
                    ui.label(
                        egui::RichText::new(format!("{}", res.total_minerals))
                            .color(egui::Color32::from_rgb(0, 180, 255)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{}", res.total_gas))
                            .color(egui::Color32::from_rgb(0, 200, 80)),
                    );
                    ui.end_row();
                }
            });

        // ─── Production timeline (Gantt-style) ──────────────────────────
        ui.add_space(16.0);
        ui.heading("Production Timeline");
        ui.separator();
        ui.add_space(4.0);

        let game_duration = replay.duration_secs;

        for (pid, name, spans) in &cached.production_spans {
            let player = replay.players.iter().find(|p| p.player_id == *pid);
            let color = player
                .map(|p| p.color.to_egui())
                .unwrap_or(egui::Color32::WHITE);

            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} ({} unit types)", name, spans.len()))
                    .color(color)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                let available_width = ui.available_width().max(200.0);
                let label_width = 120.0;
                let bar_width = available_width - label_width - 80.0;

                for span in spans {
                    ui.horizontal(|ui| {
                        let label_style = if span.is_building {
                            egui::RichText::new(&span.unit_name)
                                .color(egui::Color32::LIGHT_BLUE)
                                .small()
                        } else {
                            egui::RichText::new(&span.unit_name).small()
                        };
                        ui.allocate_ui([label_width, 16.0].into(), |ui| {
                            ui.label(label_style);
                        });

                        // Draw the Gantt bar
                        let start_frac = span.first_time_secs / game_duration;
                        let end_frac = span.last_time_secs / game_duration;
                        let bar_start = start_frac as f32 * bar_width;
                        let bar_end = (end_frac as f32 * bar_width).max(bar_start + 4.0);

                        let (rect, _) =
                            ui.allocate_exact_size([bar_width, 12.0].into(), egui::Sense::hover());

                        let painter = ui.painter();
                        // Background track
                        painter.rect_filled(rect, 2.0, egui::Color32::from_gray(40));
                        // Active bar
                        let bar_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.min.x + bar_start, rect.min.y),
                            egui::pos2(rect.min.x + bar_end, rect.max.y),
                        );
                        painter.rect_filled(bar_rect, 2.0, color);

                        // Count label
                        ui.label(
                            egui::RichText::new(format!("x{}", span.count))
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    });
                }
            });
            ui.add_space(8.0);
        }

        // ─── Idle time / Macro gap analysis ─────────────────────────────
        ui.add_space(8.0);
        ui.heading("Idle Time / Macro Gaps");
        ui.separator();
        ui.add_space(4.0);

        for (pid, name, idle) in &cached.idle_analyses {
            let player = replay.players.iter().find(|p| p.player_id == *pid);
            let color = player
                .map(|p| p.color.to_egui())
                .unwrap_or(egui::Color32::WHITE);

            egui::CollapsingHeader::new(
                egui::RichText::new(format!(
                    "{} — {} gaps, {:.0}s total idle",
                    name, idle.gap_count, idle.total_idle_secs
                ))
                .color(color)
                .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Longest gap:");
                    ui.label(
                        egui::RichText::new(format!("{:.1}s", idle.longest_gap_secs)).color(
                            if idle.longest_gap_secs > 15.0 {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::GRAY
                            },
                        ),
                    );
                });

                if !idle.gaps.is_empty() {
                    egui::Grid::new(format!("idle_{}", pid))
                        .num_columns(3)
                        .spacing([16.0, 3.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Start").strong());
                            ui.label(egui::RichText::new("End").strong());
                            ui.label(egui::RichText::new("Duration").strong());
                            ui.end_row();

                            for gap in idle.gaps.iter().take(20) {
                                let start_m = (gap.start_secs / 60.0) as u32;
                                let start_s = (gap.start_secs % 60.0) as u32;
                                let end_m = (gap.end_secs / 60.0) as u32;
                                let end_s = (gap.end_secs % 60.0) as u32;

                                ui.label(
                                    egui::RichText::new(format!("{}:{:02}", start_m, start_s))
                                        .monospace()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}:{:02}", end_m, end_s))
                                        .monospace()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}s", gap.duration_secs))
                                        .color(if gap.duration_secs > 10.0 {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        } else if gap.duration_secs > 7.0 {
                                            egui::Color32::YELLOW
                                        } else {
                                            egui::Color32::GRAY
                                        }),
                                );
                                ui.end_row();
                            }
                            if idle.gaps.len() > 20 {
                                ui.label("");
                                ui.label("");
                                ui.label(
                                    egui::RichText::new(format!(
                                        "... and {} more",
                                        idle.gaps.len() - 20
                                    ))
                                    .color(egui::Color32::GRAY),
                                );
                                ui.end_row();
                            }
                        });
                }
            });
            ui.add_space(8.0);
        }

        // ─── CSV Export button ───────────────────────────────────────────
        ui.add_space(16.0);
        ui.separator();
        if ui.button("Export to CSV").clicked() {
            self.export_csv(replay);
        }
    }

    fn export_csv(&self, replay: &Replay) {
        let cached = match &self.cached {
            Some(c) => c,
            None => return,
        };

        let csv = analytics::export_csv(
            replay,
            &cached.apm_data,
            &cached.build_orders,
            &cached.unit_counts,
            &cached.resource_estimates,
            &cached.idle_analyses,
        );

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("replay_stats.csv")
            .save_file()
        {
            let _ = std::fs::write(path, csv);
        }
    }

    fn render_batch(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("Multi-Replay Batch View");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("Load Folder").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.batch_replays.clear();
                    self.batch_selected = None;
                    if let Ok(entries) = std::fs::read_dir(&folder) {
                        let mut rep_files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .map(|ext| ext == "rep" || ext == "Rep" || ext == "REP")
                                    .unwrap_or(false)
                            })
                            .collect();
                        rep_files.sort_by_key(|e| e.file_name());

                        for entry in &rep_files {
                            if let Ok(data) = std::fs::read(entry.path()) {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                self.load_replay_batch(filename, data);
                            }
                        }
                    }
                }
            }
            if !self.batch_replays.is_empty() {
                ui.label(format!("{} replays loaded", self.batch_replays.len()));
                if ui.button("Clear").clicked() {
                    self.batch_replays.clear();
                    self.batch_selected = None;
                }
            }
        });

        if self.batch_replays.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label("Click 'Load Folder' to load all .rep files from a directory");
            });
            return;
        }

        ui.add_space(8.0);

        // Batch summary table
        egui::Grid::new("batch_table")
            .num_columns(6)
            .spacing([12.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("File").strong());
                ui.label(egui::RichText::new("Map").strong());
                ui.label(egui::RichText::new("Matchup").strong());
                ui.label(egui::RichText::new("Duration").strong());
                ui.label(egui::RichText::new("Date").strong());
                ui.label(egui::RichText::new("").strong());
                ui.end_row();

                for (i, entry) in self.batch_replays.iter().enumerate() {
                    let selected = self.batch_selected == Some(i);
                    let text_color = if selected {
                        egui::Color32::from_rgb(0, 200, 255)
                    } else {
                        egui::Color32::WHITE
                    };

                    ui.label(
                        egui::RichText::new(&entry.filename)
                            .color(text_color)
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(&entry.replay.map_name)
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(egui::RichText::new(&entry.replay.matchup).small());
                    let mins = (entry.replay.duration_secs / 60.0) as u32;
                    let secs = (entry.replay.duration_secs % 60.0) as u32;
                    ui.label(
                        egui::RichText::new(format!("{}:{:02}", mins, secs))
                            .monospace()
                            .small(),
                    );
                    let dt = chrono::DateTime::from_timestamp(entry.replay.timestamp, 0);
                    let date_str = match dt {
                        Some(d) => d.format("%Y-%m-%d").to_string(),
                        None => "-".to_string(),
                    };
                    ui.label(
                        egui::RichText::new(date_str)
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    if ui.small_button("View").clicked() {
                        self.batch_selected = Some(i);
                    }
                    ui.end_row();
                }
            });

        // Show selected replay's summary
        if let Some(idx) = self.batch_selected {
            if idx < self.batch_replays.len() {
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                let entry = &self.batch_replays[idx];
                ui.heading(format!("{} — {}", entry.replay.matchup, entry.filename));

                // Quick stats
                egui::Grid::new("batch_detail")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        for (_, name, apm) in &entry.cached.apm_data {
                            ui.label(egui::RichText::new(name).strong());
                            ui.label(format!(
                                "APM: {:.0} | EAPM: {:.0}",
                                apm.avg_apm, apm.avg_eapm
                            ));
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                egui::Grid::new("batch_resources")
                    .num_columns(3)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Player").strong());
                        ui.label(egui::RichText::new("Minerals").strong());
                        ui.label(egui::RichText::new("Gas").strong());
                        ui.end_row();

                        for (_, name, res) in &entry.cached.resource_estimates {
                            ui.label(name);
                            ui.label(format!("{}", res.total_minerals));
                            ui.label(format!("{}", res.total_gas));
                            ui.end_row();
                        }
                    });
            }
        }

        // Batch export
        ui.add_space(16.0);
        ui.separator();
        if ui.button("Export All to CSV").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name("batch_stats.csv")
                .save_file()
            {
                let mut csv = String::from("File,Map,Matchup,Duration (s),Date");
                // Add player APM columns dynamically
                csv.push_str(",P1 Name,P1 APM,P1 EAPM,P1 Minerals,P1 Gas");
                csv.push_str(",P2 Name,P2 APM,P2 EAPM,P2 Minerals,P2 Gas\n");

                for entry in &self.batch_replays {
                    csv.push_str(&format!(
                        "{},{},{},{:.0},{}",
                        escape_csv(&entry.filename),
                        escape_csv(&entry.replay.map_name),
                        entry.replay.matchup,
                        entry.replay.duration_secs,
                        entry.replay.timestamp,
                    ));

                    // Player 1 stats
                    for p_idx in 0..2 {
                        if let Some((_, name, apm)) = entry.cached.apm_data.get(p_idx) {
                            let res = entry
                                .cached
                                .resource_estimates
                                .get(p_idx)
                                .map(|(_, _, r)| r);
                            csv.push_str(&format!(
                                ",{},{:.0},{:.0},{},{}",
                                escape_csv(name),
                                apm.avg_apm,
                                apm.avg_eapm,
                                res.map(|r| r.total_minerals).unwrap_or(0),
                                res.map(|r| r.total_gas).unwrap_or(0),
                            ));
                        } else {
                            csv.push_str(",,,,");
                        }
                    }
                    csv.push('\n');
                }

                let _ = std::fs::write(path, csv);
            }
        }
    }

    fn render_logs(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("Logs");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.log_entries.clear();
                    self.log(LogLevel::Info, "Logs cleared");
                }
                if ui.button("Copy All").clicked() {
                    let text: String = self
                        .log_entries
                        .iter()
                        .map(|e| {
                            let tag = match e.level {
                                LogLevel::Info => "INFO",
                                LogLevel::Warn => "WARN",
                                LogLevel::Error => "ERR ",
                            };
                            format!("[{:>8.2}s] {} {}", e.elapsed_secs, tag, e.message)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.output_mut(|o| o.copied_text = text);
                }
                ui.checkbox(&mut self.log_auto_scroll, "Auto-scroll");
            });
        });
        ui.separator();

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
        let num_rows = self.log_entries.len();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.log_auto_scroll)
            .show_rows(ui, row_height, num_rows, |ui, row_range| {
                for i in row_range {
                    let entry = &self.log_entries[i];
                    let (tag, color) = match entry.level {
                        LogLevel::Info => ("INFO", egui::Color32::from_rgb(180, 180, 180)),
                        LogLevel::Warn => ("WARN", egui::Color32::from_rgb(255, 200, 50)),
                        LogLevel::Error => ("ERR ", egui::Color32::from_rgb(255, 100, 100)),
                    };
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "[{:>8.2}s] {} {}",
                                entry.elapsed_secs, tag, entry.message
                            ))
                            .monospace()
                            .color(color),
                        );
                    });
                }
            });
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
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
            self.log(LogLevel::Info, format!("File dropped ({} bytes)", data.len()));
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
                        self.log(LogLevel::Info, format!("Opening file: {}", path.display()));
                        match std::fs::read(&path) {
                            Ok(data) => self.load_replay(data),
                            Err(e) => {
                                let msg = format!("Failed to read file: {}", e);
                                self.log(LogLevel::Error, &msg);
                                self.error = Some(msg);
                                self.replay = None;
                                self.cached = None;
                            }
                        }
                    }
                }

                // Tab buttons
                ui.separator();
                if self.replay.is_some() {
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
                    if ui
                        .selectable_label(self.active_tab == Tab::Analytics, "Analytics")
                        .clicked()
                    {
                        self.active_tab = Tab::Analytics;
                    }
                }
                // Batch and Logs tabs always available
                if ui
                    .selectable_label(self.active_tab == Tab::Batch, "Batch")
                    .clicked()
                {
                    self.active_tab = Tab::Batch;
                }
                if ui
                    .selectable_label(self.active_tab == Tab::Logs, "Logs")
                    .clicked()
                {
                    self.active_tab = Tab::Logs;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_tab == Tab::Logs {
                self.render_logs(ui);
            } else if self.active_tab == Tab::Batch {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_batch(ui);
                });
            } else if let Some(ref error) = self.error {
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
                    Tab::Analytics => self.render_analytics(ui, &replay),
                    Tab::Batch | Tab::Logs => {} // handled above
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
