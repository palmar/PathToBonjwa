use std::time::Instant;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::analytics::{
    self, ApmData, BuildOrderEntry, HotkeyStats, IdleAnalysis, ResourceEstimate, SupplyCurve,
    UnitCount, UnitProductionSpan,
};
use crate::parser::{self, Replay};

// ─── BW-inspired color palette ──────────────────────────────────────────────
/// Deep space black — main background
const BW_BG: egui::Color32 = egui::Color32::from_rgb(8, 12, 18);
/// Slightly lighter panel background
const BW_PANEL: egui::Color32 = egui::Color32::from_rgb(14, 20, 30);
/// Panel/header darker stripe
const BW_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(10, 15, 24);
/// Teal accent — primary highlight (BW console feel)
const BW_TEAL: egui::Color32 = egui::Color32::from_rgb(0, 180, 160);
/// Brighter teal for hover/active
const BW_TEAL_BRIGHT: egui::Color32 = egui::Color32::from_rgb(0, 220, 200);
/// Cyan accent for interactive elements
const BW_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 200, 255);
/// Muted border color
const BW_BORDER: egui::Color32 = egui::Color32::from_rgb(30, 50, 60);
/// Bright border for focused elements
const BW_BORDER_BRIGHT: egui::Color32 = egui::Color32::from_rgb(40, 80, 90);
/// Default text — slightly blue-tinted white
const BW_TEXT: egui::Color32 = egui::Color32::from_rgb(180, 200, 210);
/// Dim text for secondary info
const BW_TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(100, 120, 130);
/// Header/title text
const BW_TEXT_HEADING: egui::Color32 = egui::Color32::from_rgb(0, 210, 190);

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

/// Draw a BW-styled section heading with a teal accent bar on the left.
fn bw_section_heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        // Teal accent bar
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 18.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 1.0, BW_TEAL);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(15.0)
                .color(BW_TEXT_HEADING),
        );
    });
    // Thin separator line
    let rect = ui.available_rect_before_wrap();
    let line_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0));
    ui.painter().rect_filled(line_rect, 0.0, BW_BORDER);
    ui.add_space(4.0);
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

            // Title with BW glow feel
            ui.label(
                egui::RichText::new("PathToBonjwa")
                    .strong()
                    .size(28.0)
                    .color(BW_TEAL_BRIGHT),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("BROOD WAR REPLAY ANALYZER")
                    .size(11.0)
                    .color(BW_TEXT_DIM)
                    .monospace(),
            );

            ui.add_space(8.0);
            // Decorative line
            let rect = ui.available_rect_before_wrap();
            let center_x = rect.center().x;
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(center_x - 80.0, rect.min.y),
                egui::vec2(160.0, 1.0),
            );
            ui.painter().rect_filled(line_rect, 0.0, BW_TEAL);
            ui.add_space(8.0);

            ui.add_space(30.0);
            ui.label(
                egui::RichText::new("Drop a .rep file here or click Open to load a replay")
                    .color(BW_TEXT),
            );
            ui.add_space(16.0);
        });
    }

    fn render_summary(&self, ui: &mut egui::Ui, replay: &Replay) {
        ui.add_space(8.0);

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
        bw_section_heading(ui, "Unit Production");

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

        // ─── Supply curve chart ──────────────────────────────────────────
        ui.add_space(16.0);
        bw_section_heading(ui, "Supply Over Time");

        Plot::new("supply_chart")
            .height(plot_height)
            .allow_scroll(false)
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
        bw_section_heading(ui, "Cumulative Resource Spending");

        Plot::new("resource_chart")
            .height(plot_height)
            .allow_scroll(false)
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
        bw_section_heading(ui, "Resource Estimates");

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
        bw_section_heading(ui, "Production Timeline");

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

        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::NONE
                    .fill(BW_PANEL)
                    .inner_margin(egui::Margin::symmetric(8, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("PathToBonjwa")
                            .strong()
                            .size(16.0)
                            .color(BW_TEAL),
                    );
                    ui.label(
                        egui::RichText::new("BW Replay Analyzer")
                            .size(10.0)
                            .color(BW_TEXT_DIM),
                    );
                    ui.add_space(8.0);
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
                });

                // ─── Tab bar ─────────────────────────────────────────────
                ui.add_space(2.0);
                // Draw a thin teal line above the tabs
                let rect = ui.available_rect_before_wrap();
                let line_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.0));
                ui.painter().rect_filled(line_rect, 0.0, BW_BORDER);
                ui.add_space(4.0);

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
                                .size(13.0)
                        } else {
                            egui::RichText::new(label).color(BW_TEXT_DIM).size(13.0)
                        };

                        let resp = ui.selectable_label(is_active, text);

                        // Draw active indicator line under selected tab
                        if is_active {
                            let tab_rect = resp.rect;
                            let indicator = egui::Rect::from_min_size(
                                egui::pos2(tab_rect.min.x, tab_rect.max.y - 2.0),
                                egui::vec2(tab_rect.width(), 2.0),
                            );
                            ui.painter().rect_filled(indicator, 0.0, BW_TEAL);
                        }

                        if resp.clicked() {
                            self.active_tab = tab;
                        }
                    }
                });
            });

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
