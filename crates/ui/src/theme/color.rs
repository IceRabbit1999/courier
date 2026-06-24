use std::sync::RwLock;

use egui::Color32;
use serde::{Deserialize, Serialize};

static COLORS: RwLock<ColorPalette> = RwLock::new(ColorPalette::DARK);

pub fn colors() -> ColorPalette {
    COLORS.read().map(|c| c.clone()).unwrap_or(ColorPalette::DARK)
}

pub fn set_colors(palette: ColorPalette) {
    if let Ok(mut colors) = COLORS.write() {
        *colors = palette;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SerializableColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl SerializableColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_color32(&self) -> Color32 {
        Color32::from_rgb(self.r, self.g, self.b)
    }
}

impl From<SerializableColor> for Color32 {
    fn from(c: SerializableColor) -> Self {
        c.to_color32()
    }
}

impl From<Color32> for SerializableColor {
    fn from(c: Color32) -> Self {
        Self { r: c.r(), g: c.g(), b: c.b() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub background: SerializableColor,
    pub surface: SerializableColor,
    pub surface_hover: SerializableColor,
    pub surface_secondary: SerializableColor,

    pub text: SerializableColor,
    pub text_secondary: SerializableColor,
    pub text_muted: SerializableColor,

    pub primary: SerializableColor,
    pub primary_hover: SerializableColor,
    /// Foreground color for content sitting on top of `primary` (gold) fills.
    pub on_primary: SerializableColor,
    pub secondary: SerializableColor,

    pub success: SerializableColor,
    pub danger: SerializableColor,
    pub warning: SerializableColor,

    pub victory: SerializableColor,
    pub defeat: SerializableColor,

    pub sidebar_bg: SerializableColor,
    pub sidebar_item_hover: SerializableColor,
    pub sidebar_item_active: SerializableColor,

    pub border: SerializableColor,
    pub border_light: SerializableColor,

    pub gradient_start: SerializableColor,
    pub gradient_end: SerializableColor,
    pub accent: SerializableColor,
    pub accent_muted: SerializableColor,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::dark()
    }
}

impl ThemeConfig {
    pub fn dark() -> Self {
        Self {
            // Warm graphite, layered from deepest (sidebar) to lightest (surface).
            background: SerializableColor::new(23, 24, 27),        // #17181b
            surface: SerializableColor::new(31, 33, 38),           // #1f2126
            surface_hover: SerializableColor::new(42, 44, 50),     // #2a2c32
            surface_secondary: SerializableColor::new(37, 39, 44), // #25272c

            text: SerializableColor::new(236, 237, 238),           // #ecedee
            text_secondary: SerializableColor::new(166, 169, 176), // #a6a9b0
            text_muted: SerializableColor::new(113, 117, 124),     // #71757c

            // Gold accent. Dark text rides on top of it (see on_primary).
            primary: SerializableColor::new(224, 168, 62),       // #e0a83e
            primary_hover: SerializableColor::new(201, 146, 47), // #c9922f
            on_primary: SerializableColor::new(24, 25, 28),      // #18191c
            secondary: SerializableColor::new(110, 140, 168),    // #6e8ca8

            success: SerializableColor::new(66, 178, 124), // #42b27c
            danger: SerializableColor::new(221, 90, 82),   // #dd5a52
            warning: SerializableColor::new(224, 138, 60), // #e08a3c

            victory: SerializableColor::new(66, 178, 124), // #42b27c
            defeat: SerializableColor::new(221, 90, 82),   // #dd5a52

            sidebar_bg: SerializableColor::new(19, 20, 23),          // #131417
            sidebar_item_hover: SerializableColor::new(31, 33, 38),  // #1f2126
            sidebar_item_active: SerializableColor::new(43, 45, 51), // #2b2d33

            border: SerializableColor::new(45, 47, 53),       // #2d2f35
            border_light: SerializableColor::new(35, 37, 42), // #23252a

            gradient_start: SerializableColor::new(224, 168, 62), // #e0a83e
            gradient_end: SerializableColor::new(201, 146, 47),   // #c9922f
            accent: SerializableColor::new(224, 168, 62),         // #e0a83e
            accent_muted: SerializableColor::new(138, 110, 46),   // #8a6e2e
        }
    }

    pub fn light() -> Self {
        Self {
            // Warm paper with a darker bronze accent (gold lacks contrast on white).
            background: SerializableColor::new(246, 246, 244),        // #f6f6f4
            surface: SerializableColor::new(255, 255, 255),           // #ffffff
            surface_hover: SerializableColor::new(237, 237, 233),     // #ededec
            surface_secondary: SerializableColor::new(240, 240, 237), // #f0f0ed

            text: SerializableColor::new(27, 28, 31),           // #1b1c1f
            text_secondary: SerializableColor::new(86, 89, 97), // #565961
            text_muted: SerializableColor::new(138, 141, 148),  // #8a8d94

            primary: SerializableColor::new(176, 122, 28),       // #b07a1c
            primary_hover: SerializableColor::new(150, 102, 18), // #966612
            on_primary: SerializableColor::new(255, 255, 255),   // #ffffff
            secondary: SerializableColor::new(90, 120, 150),     // #5a7896

            success: SerializableColor::new(46, 158, 104), // #2e9e68
            danger: SerializableColor::new(209, 70, 64),   // #d14640
            warning: SerializableColor::new(201, 120, 30), // #c9781e

            victory: SerializableColor::new(46, 158, 104), // #2e9e68
            defeat: SerializableColor::new(209, 70, 64),   // #d14640

            sidebar_bg: SerializableColor::new(240, 240, 236),          // #f0f0ec
            sidebar_item_hover: SerializableColor::new(231, 231, 226),  // #e7e7e2
            sidebar_item_active: SerializableColor::new(220, 218, 209), // #dcdad1

            border: SerializableColor::new(226, 226, 221),       // #e2e2dd
            border_light: SerializableColor::new(236, 236, 232), // #ecece8

            gradient_start: SerializableColor::new(176, 122, 28), // #b07a1c
            gradient_end: SerializableColor::new(150, 102, 18),   // #966612
            accent: SerializableColor::new(176, 122, 28),         // #b07a1c
            accent_muted: SerializableColor::new(190, 168, 120),  // #bea878
        }
    }

    pub fn to_palette(&self) -> ColorPalette {
        ColorPalette {
            background: self.background.to_color32(),
            surface: self.surface.to_color32(),
            surface_hover: self.surface_hover.to_color32(),
            surface_secondary: self.surface_secondary.to_color32(),

            text: self.text.to_color32(),
            text_secondary: self.text_secondary.to_color32(),
            text_muted: self.text_muted.to_color32(),

            primary: self.primary.to_color32(),
            primary_hover: self.primary_hover.to_color32(),
            on_primary: self.on_primary.to_color32(),
            secondary: self.secondary.to_color32(),

            success: self.success.to_color32(),
            danger: self.danger.to_color32(),
            warning: self.warning.to_color32(),

            victory: self.victory.to_color32(),
            defeat: self.defeat.to_color32(),

            sidebar_bg: self.sidebar_bg.to_color32(),
            sidebar_item_hover: self.sidebar_item_hover.to_color32(),
            sidebar_item_active: self.sidebar_item_active.to_color32(),

            border: self.border.to_color32(),
            border_light: self.border_light.to_color32(),

            gradient_start: self.gradient_start.to_color32(),
            gradient_end: self.gradient_end.to_color32(),
            accent: self.accent.to_color32(),
            accent_muted: self.accent_muted.to_color32(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub background: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_secondary: Color32,

    pub text: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,

    pub primary: Color32,
    pub primary_hover: Color32,
    pub on_primary: Color32,
    pub secondary: Color32,

    pub success: Color32,
    pub danger: Color32,
    pub warning: Color32,

    pub victory: Color32,
    pub defeat: Color32,

    pub sidebar_bg: Color32,
    pub sidebar_item_hover: Color32,
    pub sidebar_item_active: Color32,

    pub border: Color32,
    pub border_light: Color32,

    pub gradient_start: Color32,
    pub gradient_end: Color32,
    pub accent: Color32,
    pub accent_muted: Color32,
}

impl ColorPalette {
    // Graphite + gold dark theme.
    pub const DARK: Self = Self {
        background: Color32::from_rgb(23, 24, 27),        // #17181b
        surface: Color32::from_rgb(31, 33, 38),           // #1f2126
        surface_hover: Color32::from_rgb(42, 44, 50),     // #2a2c32
        surface_secondary: Color32::from_rgb(37, 39, 44), // #25272c

        text: Color32::from_rgb(236, 237, 238),           // #ecedee
        text_secondary: Color32::from_rgb(166, 169, 176), // #a6a9b0
        text_muted: Color32::from_rgb(113, 117, 124),     // #71757c

        primary: Color32::from_rgb(224, 168, 62),       // #e0a83e
        primary_hover: Color32::from_rgb(201, 146, 47), // #c9922f
        on_primary: Color32::from_rgb(24, 25, 28),      // #18191c
        secondary: Color32::from_rgb(110, 140, 168),    // #6e8ca8

        success: Color32::from_rgb(66, 178, 124), // #42b27c
        danger: Color32::from_rgb(221, 90, 82),   // #dd5a52
        warning: Color32::from_rgb(224, 138, 60), // #e08a3c

        victory: Color32::from_rgb(66, 178, 124), // #42b27c
        defeat: Color32::from_rgb(221, 90, 82),   // #dd5a52

        sidebar_bg: Color32::from_rgb(19, 20, 23),          // #131417
        sidebar_item_hover: Color32::from_rgb(31, 33, 38),  // #1f2126
        sidebar_item_active: Color32::from_rgb(43, 45, 51), // #2b2d33

        border: Color32::from_rgb(45, 47, 53),       // #2d2f35
        border_light: Color32::from_rgb(35, 37, 42), // #23252a

        gradient_start: Color32::from_rgb(224, 168, 62), // #e0a83e
        gradient_end: Color32::from_rgb(201, 146, 47),   // #c9922f
        accent: Color32::from_rgb(224, 168, 62),         // #e0a83e
        accent_muted: Color32::from_rgb(138, 110, 46),   // #8a6e2e
    };

    // Warm paper + bronze light theme.
    pub const LIGHT: Self = Self {
        background: Color32::from_rgb(246, 246, 244),        // #f6f6f4
        surface: Color32::WHITE,                             // #ffffff
        surface_hover: Color32::from_rgb(237, 237, 233),     // #ededec
        surface_secondary: Color32::from_rgb(240, 240, 237), // #f0f0ed

        text: Color32::from_rgb(27, 28, 31),           // #1b1c1f
        text_secondary: Color32::from_rgb(86, 89, 97), // #565961
        text_muted: Color32::from_rgb(138, 141, 148),  // #8a8d94

        primary: Color32::from_rgb(176, 122, 28),       // #b07a1c
        primary_hover: Color32::from_rgb(150, 102, 18), // #966612
        on_primary: Color32::WHITE,                     // #ffffff
        secondary: Color32::from_rgb(90, 120, 150),     // #5a7896

        success: Color32::from_rgb(46, 158, 104), // #2e9e68
        danger: Color32::from_rgb(209, 70, 64),   // #d14640
        warning: Color32::from_rgb(201, 120, 30), // #c9781e

        victory: Color32::from_rgb(46, 158, 104), // #2e9e68
        defeat: Color32::from_rgb(209, 70, 64),   // #d14640

        sidebar_bg: Color32::from_rgb(240, 240, 236),          // #f0f0ec
        sidebar_item_hover: Color32::from_rgb(231, 231, 226),  // #e7e7e2
        sidebar_item_active: Color32::from_rgb(220, 218, 209), // #dcdad1

        border: Color32::from_rgb(226, 226, 221),       // #e2e2dd
        border_light: Color32::from_rgb(236, 236, 232), // #ecece8

        gradient_start: Color32::from_rgb(176, 122, 28), // #b07a1c
        gradient_end: Color32::from_rgb(150, 102, 18),   // #966612
        accent: Color32::from_rgb(176, 122, 28),         // #b07a1c
        accent_muted: Color32::from_rgb(190, 168, 120),  // #bea878
    };

    pub fn from_config(config: &ThemeConfig) -> Self {
        config.to_palette()
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::DARK
    }
}

pub mod spacing {
    pub const TINY: f32 = 4.0;
    pub const SMALL: f32 = 8.0;
    pub const MEDIUM: f32 = 16.0;
    pub const LARGE: f32 = 24.0;
    pub const XLARGE: f32 = 32.0;
    pub const XXLARGE: f32 = 48.0;
}

pub mod radius {
    pub const SMALL: u8 = 6;
    pub const MEDIUM: u8 = 10;
    pub const LARGE: u8 = 14;
    pub const ROUND: u8 = 255;
}

pub mod font_size {
    pub const SMALL: f32 = 12.0;
    pub const BODY: f32 = 14.0;
    pub const MEDIUM: f32 = 15.0;
    pub const LARGE: f32 = 18.0;
    pub const XLARGE: f32 = 22.0;
    pub const TITLE: f32 = 28.0;
}

pub mod sidebar {
    pub fn width() -> f32 {
        configs::read().appearance.sidebar_width
    }

    pub fn collapsed_width() -> f32 {
        configs::read().appearance.sidebar_collapsed_width
    }

    pub fn item_height() -> f32 {
        configs::read().appearance.sidebar_item_height
    }

    pub fn icon_size() -> f32 {
        configs::read().appearance.sidebar_icon_size
    }
}

pub mod animation {
    pub fn sidebar_speed() -> f32 {
        configs::read().appearance.animation_sidebar_speed
    }

    pub fn toast_speed() -> f32 {
        configs::read().appearance.animation_toast_speed
    }

    pub fn hover_speed() -> f32 {
        configs::read().appearance.animation_hover_speed
    }
}
