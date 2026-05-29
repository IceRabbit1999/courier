#[cfg(feature = "inspect")]
use std::collections::VecDeque;
#[cfg(feature = "inspect")]
use std::time::Instant;

#[cfg(feature = "inspect")]
use egui::Vec2;
#[cfg(feature = "inspect")]
use egui::output::OutputEvent;

#[cfg(feature = "inspect")]
use crate::theme::{colors, font_size, radius, spacing};

#[cfg(feature = "inspect")]
const FRAME_HISTORY_LEN: usize = 120;

#[cfg(feature = "inspect")]
pub struct InspectPanel {
    visible: bool,

    sys: sysinfo::System,
    pid: sysinfo::Pid,
    last_refresh: Instant,
    process_cpu: f32,
    global_cpu: f32,
    process_memory: u64,
    total_memory: u64,

    frame_times: VecDeque<f32>,
    last_frame: Instant,

    output_events: VecDeque<String>,
    show_events: bool,
    show_egui_inspection: bool,
    show_egui_texture: bool,
    show_egui_memory: bool,
}

#[cfg(feature = "inspect")]
impl InspectPanel {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        #[allow(clippy::expect_used, reason = "dev-only inspect panel, PID must be available")]
        let pid = sysinfo::get_current_pid().expect("failed to get current pid");

        sys.refresh_memory();
        sys.refresh_cpu_usage();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);

        Self {
            visible: false,
            sys,
            pid,
            last_refresh: Instant::now(),
            process_cpu: 0.0,
            global_cpu: 0.0,
            process_memory: 0,
            total_memory: 0,
            frame_times: VecDeque::with_capacity(FRAME_HISTORY_LEN),
            last_frame: Instant::now(),
            output_events: VecDeque::with_capacity(32),
            show_events: false,
            show_egui_inspection: false,
            show_egui_texture: false,
            show_egui_memory: false,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn refresh_if_needed(&mut self) {
        if self.last_refresh.elapsed() >= sysinfo::MINIMUM_CPU_UPDATE_INTERVAL {
            self.sys.refresh_cpu_usage();
            self.sys.refresh_memory();
            self.sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), true);

            self.global_cpu = self.sys.global_cpu_usage();
            self.total_memory = self.sys.total_memory();

            if let Some(proc) = self.sys.process(self.pid) {
                self.process_cpu = proc.cpu_usage();
                self.process_memory = proc.memory();
            }

            self.last_refresh = Instant::now();
        }
    }

    fn record_frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        if self.frame_times.len() >= FRAME_HISTORY_LEN {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(dt);
    }

    fn mean_frame_time_ms(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.frame_times.iter().sum();
        (sum / self.frame_times.len() as f32) * 1000.0
    }

    fn collect_output_events(&mut self, ctx: &egui::Context) {
        let events = ctx.output(|o| o.events.clone());
        for event in events {
            let desc = match &event {
                OutputEvent::Clicked(info) => format!("Clicked: {}", info.description()),
                OutputEvent::DoubleClicked(info) => format!("DoubleClicked: {}", info.description()),
                OutputEvent::TripleClicked(info) => format!("TripleClicked: {}", info.description()),
                OutputEvent::FocusGained(info) => format!("FocusGained: {}", info.description()),
                OutputEvent::TextSelectionChanged(info) => format!("TextSelection: {}", info.description()),
                OutputEvent::ValueChanged(info) => format!("ValueChanged: {}", info.description()),
            };
            if self.output_events.len() >= 32 {
                self.output_events.pop_front();
            }
            self.output_events.push_back(desc);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.toggle();
        }

        if !self.visible {
            return;
        }

        self.record_frame();
        self.refresh_if_needed();
        self.collect_output_events(ctx);

        let palette = colors();
        let frame_time = ctx.input(|i| i.stable_dt);
        let fps = if frame_time > 0.0 { 1.0 / frame_time } else { 0.0 };
        let mean_ms = self.mean_frame_time_ms();

        egui::Window::new("Inspect")
            .collapsible(true)
            .resizable(true)
            .default_size(Vec2::new(320.0, 400.0))
            .anchor(egui::Align2::LEFT_BOTTOM, Vec2::new(spacing::MEDIUM, -spacing::MEDIUM))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_performance(ui, &palette, fps, frame_time, mean_ms);
                    ui.add_space(spacing::SMALL);
                    self.show_system(ui, &palette);
                    ui.add_space(spacing::SMALL);
                    self.show_egui_stats(ui, ctx, &palette);
                    ui.add_space(spacing::SMALL);
                    self.show_output_events(ui, &palette);
                    ui.add_space(spacing::SMALL);
                    self.show_builtin_sections(ui, ctx);
                });
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn show_performance(&self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette, fps: f32, frame_time: f32, mean_ms: f32) {
        ui.label(egui::RichText::new("Performance").size(font_size::MEDIUM).color(palette.primary));
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("FPS:").size(font_size::SMALL).color(palette.text_secondary));
            let fps_color = if fps >= 55.0 {
                palette.success
            } else if fps >= 30.0 {
                palette.warning
            } else {
                palette.danger
            };
            ui.label(egui::RichText::new(format!("{fps:.0}")).size(font_size::SMALL).color(fps_color));
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Frame:").size(font_size::SMALL).color(palette.text_secondary));
            ui.label(egui::RichText::new(format!("{:.2} ms", frame_time * 1000.0)).size(font_size::SMALL).color(palette.text));
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Mean CPU usage:").size(font_size::SMALL).color(palette.text_secondary));
            let mean_color = if mean_ms < 16.0 {
                palette.success
            } else if mean_ms < 33.0 {
                palette.warning
            } else {
                palette.danger
            };
            ui.label(egui::RichText::new(format!("{mean_ms:.2} ms / frame")).size(font_size::SMALL).color(mean_color));
        });
    }

    fn show_system(&self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette) {
        ui.label(egui::RichText::new("System").size(font_size::MEDIUM).color(palette.primary));
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CPU:").size(font_size::SMALL).color(palette.text_secondary));
            ui.label(
                egui::RichText::new(format!("{:.1}% / {:.1}%", self.process_cpu, self.global_cpu))
                    .size(font_size::SMALL)
                    .color(palette.text),
            );
        });

        let proc_mb = self.process_memory as f64 / (1024.0 * 1024.0);
        let total_mb = self.total_memory as f64 / (1024.0 * 1024.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Memory:").size(font_size::SMALL).color(palette.text_secondary));
            ui.label(
                egui::RichText::new(format!("{proc_mb:.1} MB / {total_mb:.0} MB"))
                    .size(font_size::SMALL)
                    .color(palette.text),
            );
        });
    }

    fn show_egui_stats(&self, ui: &mut egui::Ui, ctx: &egui::Context, palette: &crate::theme::ColorPalette) {
        ui.label(egui::RichText::new("egui").size(font_size::MEDIUM).color(palette.primary));
        ui.separator();

        let tex_mngr = ctx.tex_manager();
        let guard = tex_mngr.read();
        let tex_count = guard.num_allocated();
        let tex_bytes: usize = guard.allocated().map(|(_, meta)| meta.bytes_used()).sum();
        drop(guard);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Textures:").size(font_size::SMALL).color(palette.text_secondary));
            ui.label(
                egui::RichText::new(format!("{tex_count} ({} KB)", tex_bytes / 1024))
                    .size(font_size::SMALL)
                    .color(palette.text),
            );
        });
    }

    fn show_output_events(&mut self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette) {
        let header = if self.output_events.is_empty() {
            "Output Events".to_owned()
        } else {
            format!("Output Events ({})", self.output_events.len())
        };

        let id = ui.make_persistent_id("inspect_events");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, self.show_events)
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new(header).size(font_size::MEDIUM).color(palette.primary));
            })
            .body(|ui| {
                if self.output_events.is_empty() {
                    ui.label(egui::RichText::new("No events yet").size(font_size::SMALL).color(palette.text_muted));
                } else {
                    if ui.small_button("Clear").on_hover_text("Clear all events").clicked() {
                        self.output_events.clear();
                    }

                    egui::Frame::NONE
                        .fill(palette.background)
                        .corner_radius(egui::CornerRadius::same(radius::SMALL))
                        .inner_margin(spacing::TINY)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .id_salt("events_scroll")
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for event in &self.output_events {
                                        ui.label(egui::RichText::new(event).size(font_size::SMALL).color(palette.text_muted));
                                    }
                                });
                        });
                }
            });
    }

    fn show_builtin_sections(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let id_inspection = ui.make_persistent_id("inspect_egui_inspection");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id_inspection, self.show_egui_inspection)
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new("egui Inspection").size(font_size::MEDIUM));
            })
            .body(|ui| {
                ctx.inspection_ui(ui);
            });

        let id_texture = ui.make_persistent_id("inspect_egui_texture");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id_texture, self.show_egui_texture)
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new("egui Textures").size(font_size::MEDIUM));
            })
            .body(|ui| {
                ctx.texture_ui(ui);
            });

        let id_memory = ui.make_persistent_id("inspect_egui_memory");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id_memory, self.show_egui_memory)
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new("egui Memory").size(font_size::MEDIUM));
            })
            .body(|ui| {
                ctx.memory_ui(ui);
            });
    }
}

#[cfg(feature = "inspect")]
impl Default for InspectPanel {
    fn default() -> Self {
        Self::new()
    }
}
