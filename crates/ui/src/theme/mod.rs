mod color;

pub use color::{ColorPalette, SerializableColor, ThemeConfig, animation, colors, font_size, radius, set_colors, sidebar, spacing};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    Custom,
}

impl ThemeMode {
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Custom => ThemeMode::Light,
        }
    }

    pub fn is_light(&self) -> bool {
        matches!(self, ThemeMode::Light)
    }
}

#[derive(Debug, Clone)]
pub struct CourierTheme {
    pub mode: ThemeMode,
    custom_config: Option<ThemeConfig>,
    palette: ColorPalette,
}

impl CourierTheme {
    pub fn new(mode: ThemeMode) -> Self {
        let palette = match mode {
            ThemeMode::Light => ColorPalette::LIGHT,
            ThemeMode::Dark | ThemeMode::Custom => ColorPalette::DARK,
        };

        set_colors(palette.clone());

        Self {
            mode,
            custom_config: None,
            palette,
        }
    }

    pub fn dark() -> Self {
        Self::new(ThemeMode::Dark)
    }

    pub fn light() -> Self {
        Self::new(ThemeMode::Light)
    }

    pub fn custom(config: ThemeConfig) -> Self {
        let palette = config.to_palette();
        set_colors(palette.clone());

        Self {
            mode: ThemeMode::Custom,
            custom_config: Some(config),
            palette,
        }
    }

    pub fn from_config(config: ThemeConfig, is_light: bool) -> Self {
        let palette = config.to_palette();
        set_colors(palette.clone());

        Self {
            mode: if is_light { ThemeMode::Light } else { ThemeMode::Dark },
            custom_config: Some(config),
            palette,
        }
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;

        self.palette = if let Some(config) = &self.custom_config {
            config.to_palette()
        } else {
            match mode {
                ThemeMode::Light => ColorPalette::LIGHT,
                ThemeMode::Dark | ThemeMode::Custom => ColorPalette::DARK,
            }
        };

        set_colors(self.palette.clone());
    }

    pub fn apply_config(&mut self, config: ThemeConfig) {
        self.palette = config.to_palette();
        self.custom_config = Some(config);
        self.mode = ThemeMode::Custom;
        set_colors(self.palette.clone());
    }

    pub fn palette(&self) -> &ColorPalette {
        &self.palette
    }

    pub fn custom_config(&self) -> Option<&ThemeConfig> {
        self.custom_config.as_ref()
    }

    /// Apply this theme's colors to egui visuals
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let p = &self.palette;
        let mut visuals = if self.mode.is_light() { egui::Visuals::light() } else { egui::Visuals::dark() };

        visuals.override_text_color = Some(p.text);
        visuals.panel_fill = p.background;
        visuals.window_fill = p.surface;
        visuals.extreme_bg_color = p.background;
        visuals.faint_bg_color = p.surface_secondary;

        visuals.selection.bg_fill = p.primary.linear_multiply(0.4);
        visuals.selection.stroke = egui::Stroke::new(1_f32, p.primary);

        visuals.hyperlink_color = p.primary;
        visuals.warn_fg_color = p.warning;
        visuals.error_fg_color = p.danger;

        visuals.widgets.noninteractive.bg_fill = p.surface;
        visuals.widgets.noninteractive.weak_bg_fill = p.surface_secondary;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1_f32, p.text_secondary);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5_f32, p.border_light);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(radius::MEDIUM);

        visuals.widgets.inactive.bg_fill = p.surface;
        visuals.widgets.inactive.weak_bg_fill = p.surface_secondary;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1_f32, p.text_secondary);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.5_f32, p.border);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(radius::MEDIUM);

        visuals.widgets.hovered.bg_fill = p.surface_hover;
        visuals.widgets.hovered.weak_bg_fill = p.surface_hover;
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1_f32, p.text);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1_f32, p.primary);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(radius::MEDIUM);

        visuals.widgets.active.bg_fill = p.primary;
        visuals.widgets.active.weak_bg_fill = p.primary.linear_multiply(0.8);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1_f32, egui::Color32::WHITE);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1_f32, p.primary_hover);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(radius::MEDIUM);

        visuals.widgets.open.bg_fill = p.surface_hover;
        visuals.widgets.open.weak_bg_fill = p.surface_hover;
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1_f32, p.text);
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1_f32, p.primary);
        visuals.widgets.open.corner_radius = egui::CornerRadius::same(radius::MEDIUM);

        visuals.window_stroke = egui::Stroke::new(1_f32, p.border);
        visuals.window_corner_radius = egui::CornerRadius::same(radius::LARGE);
        visuals.window_shadow = egui::Shadow {
            spread: 0,
            blur: 12,
            offset: [0, 4],
            color: egui::Color32::from_black_alpha(if self.mode.is_light() { 30 } else { 80 }),
        };

        visuals.popup_shadow = egui::Shadow {
            spread: 0,
            blur: 8,
            offset: [0, 2],
            color: egui::Color32::from_black_alpha(if self.mode.is_light() { 20 } else { 60 }),
        };

        visuals.indent_has_left_vline = false;
        visuals.striped = true;

        ctx.set_visuals(visuals);
    }

    pub fn toggle(&mut self) {
        let new_mode = self.mode.toggle();

        if self.custom_config.is_none() {
            self.palette = match new_mode {
                ThemeMode::Light => ColorPalette::LIGHT,
                ThemeMode::Dark | ThemeMode::Custom => ColorPalette::DARK,
            };
            set_colors(self.palette.clone());
        }

        self.mode = new_mode;
    }
}

impl Default for CourierTheme {
    fn default() -> Self {
        Self::dark()
    }
}
