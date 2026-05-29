use egui::{self, Color32, CornerRadius, Stroke, StrokeKind, Vec2};

use crate::{
    app::Route,
    icons::{action, nav},
    theme::{colors, font_size, radius, sidebar, spacing},
};

pub fn show(ui: &mut egui::Ui, active_route: &mut Route, collapsed: &mut bool) {
    let palette = colors();

    let width = if *collapsed { sidebar::collapsed_width() } else { sidebar::width() };

    egui::Frame::NONE
        .fill(palette.sidebar_bg)
        .inner_margin(if *collapsed { spacing::SMALL } else { spacing::MEDIUM })
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_min_height(ui.available_height());

            ui.allocate_ui_with_layout(Vec2::new(width, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                if *collapsed {
                    show_collapsed(ui, active_route, collapsed);
                } else {
                    show_expanded(ui, active_route, collapsed);
                }
            });
        });
}

fn show_expanded(ui: &mut egui::Ui, active_route: &mut Route, collapsed: &mut bool) {
    let palette = colors();

    // Header with logo and collapse button
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(i18n::message("app-name")).size(font_size::XLARGE).color(palette.text));
            ui.label(egui::RichText::new(i18n::message("app-tagline")).size(font_size::SMALL).color(palette.text_muted));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(action::PANEL_LEFT).size(16.0).color(palette.text_muted))
                        .frame(false)
                        .min_size(Vec2::new(32.0, 32.0)),
                )
                .clicked()
            {
                *collapsed = true;
            }
        });
    });

    ui.add_space(spacing::SMALL);

    // Main navigation items
    for route in Route::main_routes() {
        nav_item(ui, *route, active_route);
    }

    // Push settings to bottom
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        nav_item(ui, Route::Settings, active_route);
    });
}

fn show_collapsed(ui: &mut egui::Ui, active_route: &mut Route, collapsed: &mut bool) {
    let palette = colors();

    // Expand button
    ui.vertical_centered(|ui| {
        if ui
            .add(
                egui::Button::new(egui::RichText::new(action::MENU).size(16.0).color(palette.text_muted))
                    .frame(false)
                    .min_size(Vec2::new(32.0, 32.0)),
            )
            .clicked()
        {
            *collapsed = false;
        }
    });

    ui.add_space(spacing::SMALL);

    // Navigation items (icon only)
    for route in Route::main_routes() {
        nav_item_collapsed(ui, *route, active_route);
    }

    // Push settings to bottom
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        nav_item_collapsed(ui, Route::Settings, active_route);
    });
}

fn nav_item(ui: &mut egui::Ui, route: Route, active_route: &mut Route) {
    let palette = colors();
    let is_active = *active_route == route;
    let icon = get_route_icon(route);

    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), sidebar::item_height()), egui::Sense::click());

    if response.clicked() {
        *active_route = route;
    }

    let hover_t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered() || is_active, 1.0 / crate::theme::animation::hover_speed());

    let bg_color = if is_active {
        palette.sidebar_item_active
    } else {
        let mut c = palette.sidebar_item_hover;
        c = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (hover_t * c.a() as f32) as u8);
        c
    };

    let icon_color = if is_active {
        palette.primary
    } else {
        lerp_color(palette.text, palette.primary, hover_t)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(radius::MEDIUM), bg_color);

    if is_active {
        painter.rect_stroke(rect, CornerRadius::same(radius::MEDIUM), Stroke::new(2.0, palette.primary), StrokeKind::Outside);
    }

    let icon_pos = rect.left_center() + Vec2::new(spacing::MEDIUM, 0.0);
    painter.text(icon_pos, egui::Align2::LEFT_CENTER, icon, egui::FontId::proportional(sidebar::icon_size()), icon_color);

    let text_color = if is_active {
        palette.primary
    } else if response.hovered() {
        palette.text
    } else {
        palette.text_secondary
    };

    let label_pos = icon_pos + Vec2::new(sidebar::icon_size() + spacing::MEDIUM, 0.0);
    painter.text(label_pos, egui::Align2::LEFT_CENTER, route.title(), egui::FontId::proportional(font_size::BODY), text_color);
}

fn nav_item_collapsed(ui: &mut egui::Ui, route: Route, active_route: &mut Route) {
    let palette = colors();
    let is_active = *active_route == route;
    let icon = get_route_icon(route);
    let icon_color = if is_active { palette.primary } else { palette.text };

    let response = ui
        .vertical_centered(|ui| {
            let btn = egui::Button::new(egui::RichText::new(icon).size(sidebar::icon_size()).color(icon_color))
                .fill(if is_active { palette.sidebar_item_active } else { Color32::TRANSPARENT })
                .stroke(if is_active { Stroke::new(2.0, palette.primary) } else { Stroke::NONE })
                .corner_radius(CornerRadius::same(radius::MEDIUM))
                .min_size(Vec2::new(44.0, 44.0));

            ui.add(btn)
        })
        .inner;

    if response.clicked() {
        *active_route = route;
    }

    response.on_hover_text(route.title());
}

fn get_route_icon(route: Route) -> &'static str {
    match route {
        Route::Dashboard => nav::DASHBOARD,
        Route::Friends => nav::FRIENDS,
        Route::Matches => nav::MATCHES,
        Route::Heroes => nav::HEROES,
        Route::Items => nav::ITEMS,
        Route::Settings => nav::SETTINGS,
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}
