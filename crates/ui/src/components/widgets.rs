use egui::{Color32, CornerRadius, Margin, Stroke, UiBuilder, Vec2};
use shared::{PersonaState, RecentGame};

use crate::theme::{ColorPalette, animation, colors, font_size, radius, spacing};

/// Upper bound on content width so screens stay readable on wide windows
/// instead of stretching edge to edge.
pub const CONTENT_MAX_WIDTH: f32 = 880.0;

/// Scrollable page body: a centered column capped at [`CONTENT_MAX_WIDTH`] with
/// consistent vertical padding. Every screen wraps its content in this.
pub fn page(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let total = ui.available_width();
        let content_w = (total - spacing::XLARGE * 2.0).clamp(320.0, CONTENT_MAX_WIDTH);
        let side = ((total - content_w) / 2.0).max(spacing::LARGE);

        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.allocate_ui_with_layout(Vec2::new(content_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(content_w);
                ui.add_space(spacing::LARGE);
                add_contents(ui);
                ui.add_space(spacing::XLARGE);
            });
        });
    });
}

/// Screen title with an optional right-aligned action cluster.
pub fn screen_header(ui: &mut egui::Ui, title: &str, right: impl FnOnce(&mut egui::Ui)) {
    let p = colors();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(font_size::TITLE).strong().color(p.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
}

/// Subsection heading used inside a screen.
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    let p = colors();
    ui.label(egui::RichText::new(text).size(font_size::LARGE).strong().color(p.text));
}

/// Filled gold call-to-action. Caller handles `.clicked()` / `add_enabled`.
pub fn primary_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    primary_button_enabled(ui, text, true)
}

pub fn primary_button_enabled(ui: &mut egui::Ui, text: impl Into<String>, enabled: bool) -> egui::Response {
    let p = colors();
    let btn = egui::Button::new(egui::RichText::new(text).size(font_size::BODY).color(p.on_primary))
        .fill(p.primary)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .min_size(Vec2::new(0.0, 34.0));
    ui.add_enabled(enabled, btn)
}

/// Transparent button with a hairline border, for secondary actions.
pub fn ghost_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ghost_button_enabled(ui, text, true)
}

pub fn ghost_button_enabled(ui: &mut egui::Ui, text: impl Into<String>, enabled: bool) -> egui::Response {
    let p = colors();
    let btn = egui::Button::new(egui::RichText::new(text).size(font_size::BODY).color(p.text_secondary))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1_f32, p.border))
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .min_size(Vec2::new(0.0, 34.0));
    ui.add_enabled(enabled, btn)
}

/// A single-choice segmented control: one rounded track, the active segment
/// filled and gold-labelled. Replaces the old "2px primary outline" buttons.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, options: &[(T, String)]) {
    let p = colors();
    egui::Frame::NONE
        .fill(p.surface_secondary)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing::TINY;
                for (value, label) in options {
                    let active = *current == *value;
                    let (fill, text_color) = if active {
                        (p.surface_hover, p.primary)
                    } else {
                        (Color32::TRANSPARENT, p.text_secondary)
                    };
                    let btn = egui::Button::new(egui::RichText::new(label).size(font_size::BODY).color(text_color))
                        .fill(fill)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(radius::SMALL))
                        .min_size(Vec2::new(0.0, 28.0));
                    if ui.add(btn).clicked() {
                        *current = *value;
                    }
                }
            });
        });
}

/// An iOS-style switch. Clearer than egui's default checkbox, whose track fill
/// is `surface` and so vanishes against a settings panel. The knob slides and
/// the track tweens between states. Returns the response (`.changed()` is true
/// on toggle).
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let p = colors();
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(40.0, 22.0), egui::Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let t = ui.ctx().animate_bool_with_time(response.id, *on, 1.0 / animation::hover_speed());
    let half = rect.height() * 0.5;

    let track = Color32::from(egui::Rgba::from(p.surface_secondary) * (1.0 - t) + egui::Rgba::from(p.primary) * t);
    ui.painter().rect_filled(rect, CornerRadius::same(half as u8), track);
    ui.painter()
        .rect_stroke(rect, CornerRadius::same(half as u8), Stroke::new(1_f32, p.border), egui::StrokeKind::Inside);

    let knob_x = egui::lerp((rect.left() + half)..=(rect.right() - half), t);
    let knob = Color32::from(egui::Rgba::from(p.text_secondary) * (1.0 - t) + egui::Rgba::from(p.on_primary) * t);
    ui.painter().circle_filled(egui::pos2(knob_x, rect.center().y), half - 3.0, knob);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Paint a short rounded vertical bar at the cursor, e.g. a match's win/loss
/// marker at the left edge of a row.
pub fn accent_bar(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 34.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), color);
}

/// A bordered tile showing a big value over a muted label.
pub fn stat_tile(ui: &mut egui::Ui, value: &str, label: &str) {
    let p = colors();
    egui::Frame::NONE
        .fill(p.surface)
        .stroke(Stroke::new(1_f32, p.border))
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .show(ui, |ui| {
            ui.set_min_width(110.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(value).size(font_size::XLARGE).strong().color(p.text));
                ui.add_space(spacing::TINY);
                ui.label(egui::RichText::new(label).size(font_size::SMALL).color(p.text_muted));
            });
        });
}

/// Full-width clickable row with hover highlight and a bottom hairline divider.
/// The closure lays out the row's content left-to-right, vertically centered.
/// Use these (not bordered cards) for list items.
pub fn list_row(ui: &mut egui::Ui, height: f32, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let p = colors();
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());

    let t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("row_hover"), response.hovered(), 1.0 / animation::hover_speed());
    if t > 0.0 {
        let fill = Color32::from_rgba_unmultiplied(p.surface_hover.r(), p.surface_hover.g(), p.surface_hover.b(), (t * 255.0) as u8);
        ui.painter().rect_filled(rect, CornerRadius::same(radius::MEDIUM), fill);
    }
    ui.painter().hline(rect.x_range(), rect.max.y, Stroke::new(1_f32, p.border_light));

    let inner = rect.shrink2(Vec2::new(spacing::SMALL, 0.0));
    let mut content = ui.new_child(UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    add_contents(&mut content);

    response
}

/// Paint a filled presence dot at the current cursor in a left-to-right row.
/// When `pulse` is set, a soft halo breathes out from the dot so live
/// (online/in-game) friends read as actually present, not just colored.
pub fn presence_dot(ui: &mut egui::Ui, color: Color32, pulse: bool) {
    let size = 10.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let center = rect.center();

    if pulse {
        let phase = (ui.ctx().input(|i| i.time) as f32 * 1.5).sin() * 0.5 + 0.5;
        let halo = size * 0.5 + phase * 4.0;
        let alpha = ((1.0 - phase) * 70.0) as u8;
        ui.painter()
            .circle_filled(center, halo, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha));
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }

    ui.painter().circle_filled(center, size * 0.5, color);
}

/// A chevron drawn by hand so it tweens smoothly from pointing-right (`t = 0`,
/// collapsed) to pointing-down (`t = 1`, expanded) instead of snapping between
/// two glyphs. Shared by watch-list-style screens (Friends, Follows).
pub fn caret(ui: &mut egui::Ui, color: Color32, t: f32) -> egui::Response {
    let size = font_size::LARGE;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let center = rect.center();
    let r = size * 0.22;
    let (sin, cos) = (t * std::f32::consts::FRAC_PI_2).sin_cos();
    let rotate = |x: f32, y: f32| egui::pos2(center.x + x * cos - y * sin, center.y + x * sin + y * cos);
    let stroke = Stroke::new(1.8_f32, color);
    ui.painter().line_segment([rotate(-r, -r), rotate(r, 0.0)], stroke);
    ui.painter().line_segment([rotate(r, 0.0), rotate(-r, r)], stroke);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Format a Steam playtime (minutes) as a compact `Xh Ym` / `Xh` / `Ym` string.
pub fn format_playtime(minutes: i64) -> String {
    if minutes <= 0 {
        return "0m".to_owned();
    }
    let (hours, mins) = (minutes / 60, minutes % 60);
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

pub fn persona_state_label(state: PersonaState) -> String {
    let key = match state {
        PersonaState::Offline => "friends-status-offline",
        PersonaState::Online => "friends-status-online",
        PersonaState::Busy => "friends-status-busy",
        PersonaState::Away => "friends-status-away",
        PersonaState::Snooze => "friends-status-snooze",
        PersonaState::LookingToTrade => "friends-status-looking-to-trade",
        PersonaState::LookingToPlay => "friends-status-looking-to-play",
    };
    i18n::message(key)
}

/// The expanded panel beneath a watch-list row: the top `limit` recently played
/// games (by two-week playtime) with their two-week and lifetime hours.
pub fn recent_games_panel(ui: &mut egui::Ui, games: &[RecentGame], palette: &ColorPalette, limit: usize) {
    let mut games = games.iter().collect::<Vec<_>>();
    games.sort_by_key(|g| std::cmp::Reverse(g.playtime_2weeks));

    egui::Frame::NONE
        .fill(palette.surface_secondary)
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .outer_margin(Margin {
            left: spacing::LARGE as i8,
            top: spacing::TINY as i8,
            bottom: spacing::SMALL as i8,
            ..Default::default()
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(i18n::message("friends-recent-games"))
                    .size(font_size::SMALL)
                    .strong()
                    .color(palette.text_secondary),
            );
            ui.add_space(spacing::TINY);

            for game in games.into_iter().take(limit.max(1)) {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&game.name).size(font_size::BODY).color(palette.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let total = format!("{} {}", i18n::message("friends-playtime-total"), format_playtime(game.playtime_forever));
                        ui.label(egui::RichText::new(total).size(font_size::SMALL).color(palette.text_muted));
                        ui.add_space(spacing::SMALL);
                        let two_weeks = format!("{} {}", i18n::message("friends-playtime-2weeks"), format_playtime(game.playtime_2weeks));
                        ui.label(egui::RichText::new(two_weeks).size(font_size::SMALL).color(palette.primary));
                    });
                });
            }
        });
}
