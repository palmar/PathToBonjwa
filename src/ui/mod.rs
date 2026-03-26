use std::time::Instant;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::analytics::{self, ApmData, BuildOrderEntry, HotkeyStats, IdleAnalysis};
use crate::parser::{self, Replay};

// ─── BW-inspired color palette ──────────────────────────────────────────────
/// Deep space black — main background
const BW_BG: egui::Color32 = egui::Color32::from_rgb(10, 14, 22);
/// Slightly lighter panel background
const BW_PANEL: egui::Color32 = egui::Color32::from_rgb(16, 24, 34);
/// Panel/header darker stripe
const BW_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(12, 18, 28);
/// Menu bar background — subtle distinction from panel
const BW_MENU_BAR: egui::Color32 = egui::Color32::from_rgb(18, 26, 38);
/// Teal accent — primary highlight (BW console feel)
const BW_TEAL: egui::Color32 = egui::Color32::from_rgb(0, 190, 170);
/// Brighter teal for hover/active
const BW_TEAL_BRIGHT: egui::Color32 = egui::Color32::from_rgb(0, 230, 210);
/// Cyan accent for interactive elements
const BW_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 200, 255);
/// Muted border color
const BW_BORDER: egui::Color32 = egui::Color32::from_rgb(34, 54, 66);
/// Bright border for focused elements
const BW_BORDER_BRIGHT: egui::Color32 = egui::Color32::from_rgb(44, 86, 96);
/// Default text — warm off-white for legibility
const BW_TEXT: egui::Color32 = egui::Color32::from_rgb(200, 215, 220);
/// Dim text for secondary info — brighter for readability
const BW_TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 142, 155);
/// Header/title text
const BW_TEXT_HEADING: egui::Color32 = egui::Color32::from_rgb(0, 220, 200);

/// Build the full egui Visuals for a Starcraft: BW retro look.
pub fn bw_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();

    // Window / panel backgrounds
    v.panel_fill = BW_BG;
    v.window_fill = BW_PANEL;
    v.faint_bg_color = BW_PANEL_DARK;
    v.extreme_bg_color = egui::Color32::from_rgb(6, 9, 14);

    // Stripes in grids
    v.striped = true;

    // Widget visuals
    let corner_radius = egui::CornerRadius::same(2);

    v.widgets.noninteractive.bg_fill = BW_PANEL;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BW_BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, BW_TEXT);
    v.widgets.noninteractive.corner_radius = corner_radius;

    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(16, 26, 36);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BW_BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, BW_TEXT);
    v.widgets.inactive.corner_radius = corner_radius;

    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(20, 36, 48);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, BW_BORDER_BRIGHT);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, BW_TEAL_BRIGHT);
    v.widgets.hovered.corner_radius = corner_radius;

    v.widgets.active.bg_fill = egui::Color32::from_rgb(0, 50, 50);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.5, BW_TEAL);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, BW_TEAL_BRIGHT);
    v.widgets.active.corner_radius = corner_radius;

    v.widgets.open.bg_fill = egui::Color32::from_rgb(14, 30, 38);
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, BW_TEAL);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, BW_TEAL_BRIGHT);
    v.widgets.open.corner_radius = corner_radius;

    // Selection
    v.selection.bg_fill = egui::Color32::from_rgb(0, 60, 55);
    v.selection.stroke = egui::Stroke::new(1.0, BW_TEAL);

    // Separator lines
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BW_BORDER);

    // Hyperlinks
    v.hyperlink_color = BW_CYAN;

    // Window shadow
    v.window_shadow = egui::Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };

    v
}

// ─── App state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Summary,
    Stats,
    Charts,
    Analytics,
    Logs,
}

#[derive(Clone)]
struct LogEntry {
    elapsed_secs: f64,
    level: LogLevel,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
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
    // Client-side logging
    log_entries: Vec<LogEntry>,
    log_start: Instant,
    log_auto_scroll: bool,
    // UI state
    show_settings: bool,
    fonts_configured: bool,
}

struct CachedAnalytics {
    apm_data: Vec<(u8, String, ApmData)>,
    build_orders: Vec<(u8, String, Vec<BuildOrderEntry>)>,
    hotkey_stats: Vec<(u8, String, HotkeyStats)>,
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
            log_entries: Vec::new(),
            log_start: start,
            log_auto_scroll: true,
            show_settings: false,
            fonts_configured: false,
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

/// Draw a BW-styled section heading with a teal accent bar on the left.
fn bw_section_heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        // Teal accent bar
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 22.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 1.0, BW_TEAL);
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(16.0)
                .color(BW_TEXT_HEADING),
        );
    });
    // Thin separator line
    let rect = ui.available_rect_before_wrap();
    let line_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0));
    ui.painter().rect_filled(line_rect, 0.0, BW_BORDER);
    ui.add_space(6.0);
}

impl App {
    fn load_replay(&mut self, data: Vec<u8>) {
        self.log(
            LogLevel::Info,
            format!("Parsing replay ({} bytes)", data.len()),
        );
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

    fn compute_analytics(replay: &Replay) -> CachedAnalytics {
        let mut apm_data = Vec::new();
        let mut build_orders = Vec::new();
        let mut hotkey_stats = Vec::new();
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
                analytics::extract_build_order(&replay.commands, pid, &player.race),
            ));
            hotkey_stats.push((
                pid,
                name.clone(),
                analytics::compute_hotkey_stats(&replay.commands, pid),
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
            hotkey_stats,
            idle_analyses,
        }
    }

    fn render_welcome(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);

            // Title with BW glow feel
            ui.label(
                egui::RichText::new("PathToBonjwa")
                    .strong()
                    .size(32.0)
                    .color(BW_TEAL_BRIGHT),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("BROOD WAR REPLAY ANALYZER")
                    .size(12.0)
                    .color(BW_TEXT_DIM)
                    .monospace(),
            );

            ui.add_space(8.0);
            // Decorative line
            let rect = ui.available_rect_before_wrap();
            let center_x = rect.center().x;
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(center_x - 100.0, rect.min.y),
                egui::vec2(200.0, 1.0),
            );
            ui.painter().rect_filled(line_rect, 0.0, BW_TEAL);
            ui.add_space(8.0);

            ui.add_space(30.0);
            ui.label(
                egui::RichText::new("Drop a .rep file here or use File > Open Replay")
                    .size(16.0)
                    .color(BW_TEXT),
            );
            ui.add_space(16.0);
        });
    }

    fn render_summary(&self, ui: &mut egui::Ui, replay: &Replay) {
        ui.add_space(10.0);

        // Matchup header — styled with BW accent
        ui.horizontal(|ui| {
            // Teal accent bar
            let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 22.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 1.0, BW_TEAL);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&replay.matchup)
                    .strong()
                    .size(18.0)
                    .color(BW_TEAL_BRIGHT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{}", replay.game_type)).color(BW_TEXT_DIM));
            });
        });
        let rect = ui.available_rect_before_wrap();
        let line_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0));
        ui.painter().rect_filled(line_rect, 0.0, BW_BORDER);
        ui.add_space(4.0);

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
        bw_section_heading(ui, "Players");

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

        // Ensure content stretches to full available width
        ui.set_min_width(ui.available_width());
        ui.add_space(8.0);

        // ─── Build orders ────────────────────────────────────────────────
        bw_section_heading(ui, "Build Orders");

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
                    .spacing([16.0, 3.0])
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

        // ─── Hotkey stats ────────────────────────────────────────────────
        ui.add_space(8.0);
        bw_section_heading(ui, "Hotkey Usage");

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
        bw_section_heading(ui, "APM Over Time");

        let plot_height = 250.0;

        // APM chart
        Plot::new("apm_chart")
            .height(plot_height)
            .allow_scroll(false)
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
        bw_section_heading(ui, "EAPM Over Time");

        Plot::new("eapm_chart")
            .height(plot_height)
            .allow_scroll(false)
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

    fn render_analytics(&self, ui: &mut egui::Ui, replay: &Replay) {
        let cached = match &self.cached {
            Some(c) => c,
            None => return,
        };

        ui.add_space(8.0);

        // ─── Idle time / Macro gap analysis ─────────────────────────────
        ui.add_space(8.0);
        bw_section_heading(ui, "Idle Time / Macro Gaps");

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

    fn render_logs(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        bw_section_heading(ui, "Logs");
        ui.horizontal(|ui| {
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
                    ui.ctx().copy_text(text);
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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure fonts and base text size on first frame
        if !self.fonts_configured {
            self.fonts_configured = true;
            let mut style = (*ctx.style()).clone();
            // Bump all text sizes slightly for better legibility
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            );
            // Slightly more padding in buttons/tabs for a larger click target
            style.spacing.button_padding = egui::vec2(10.0, 5.0);
            style.spacing.item_spacing = egui::vec2(8.0, 5.0);
            ctx.set_style(style);
        }

        // Track whether file open was triggered from the menu
        let mut open_replay = false;
        let mut quit_app = false;

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
            self.log(
                LogLevel::Info,
                format!("File dropped ({} bytes)", data.len()),
            );
            self.load_replay(data);
        }

        // ─── Menu bar ────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::NONE
                    .fill(BW_MENU_BAR)
                    .inner_margin(egui::Margin::symmetric(6, 2)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(
                        egui::RichText::new("File").size(14.0).color(BW_TEXT),
                        |ui| {
                            if ui
                                .button(egui::RichText::new("Open Replay...").size(14.0))
                                .clicked()
                            {
                                open_replay = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(egui::RichText::new("Settings").size(14.0))
                                .clicked()
                            {
                                self.show_settings = !self.show_settings;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(egui::RichText::new("Quit").size(14.0))
                                .clicked()
                            {
                                quit_app = true;
                                ui.close_menu();
                            }
                        },
                    );
                });
            });

        // ─── Header + tab bar ────────────────────────────────────────
        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::NONE
                    .fill(BW_PANEL)
                    .inner_margin(egui::Margin::symmetric(10, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("PathToBonjwa")
                            .strong()
                            .size(18.0)
                            .color(BW_TEAL),
                    );
                    ui.label(
                        egui::RichText::new("BW Replay Analyzer")
                            .size(12.0)
                            .color(BW_TEXT_DIM),
                    );
                });

                // ─── Tab bar ─────────────────────────────────────────
                ui.add_space(4.0);
                let rect = ui.available_rect_before_wrap();
                let line_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0));
                ui.painter().rect_filled(line_rect, 0.0, BW_BORDER);
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let tabs: &[(Tab, &str, bool)] = &[
                        (Tab::Summary, "Summary", self.replay.is_some()),
                        (Tab::Stats, "Stats", self.replay.is_some()),
                        (Tab::Charts, "Charts", self.replay.is_some()),
                        (Tab::Analytics, "Analytics", self.replay.is_some()),
                        (Tab::Logs, "Logs", true),
                    ];

                    for &(tab, label, enabled) in tabs {
                        if !enabled {
                            continue;
                        }
                        let is_active = self.active_tab == tab;
                        let text = if is_active {
                            egui::RichText::new(label)
                                .strong()
                                .color(BW_TEAL_BRIGHT)
                                .size(15.0)
                        } else {
                            egui::RichText::new(label).color(BW_TEXT_DIM).size(15.0)
                        };

                        let resp = ui.selectable_label(is_active, text);

                        // Draw active indicator line under selected tab
                        if is_active {
                            let tab_rect = resp.rect;
                            let indicator = egui::Rect::from_min_size(
                                egui::pos2(tab_rect.min.x, tab_rect.max.y - 2.0),
                                egui::vec2(tab_rect.width(), 2.5),
                            );
                            ui.painter().rect_filled(indicator, 0.0, BW_TEAL);
                        }

                        if resp.clicked() {
                            self.active_tab = tab;
                        }
                    }
                });
            });

        // Handle menu actions (after panel is done)
        if open_replay {
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

        if quit_app {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // ─── Settings window ─────────────────────────────────────────
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("PathToBonjwa v0.7.0")
                            .size(16.0)
                            .color(BW_TEAL),
                    );
                    ui.add_space(8.0);
                    ui.label("Settings will be available in a future update.");
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_settings = false;
                    }
                });
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(BW_BG)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                if self.active_tab == Tab::Logs {
                    self.render_logs(ui);
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
                        Tab::Logs => {} // handled above
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
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(200));
        // Teal border glow
        painter.rect_stroke(
            screen_rect.shrink(4.0),
            2,
            egui::Stroke::new(2.0, BW_TEAL),
            egui::StrokeKind::Outside,
        );

        Area::new(Id::new("drop_text"))
            .fixed_pos(screen_rect.center())
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Drop .rep file here")
                        .strong()
                        .size(20.0)
                        .color(BW_TEAL_BRIGHT),
                );
            });
    }
}
