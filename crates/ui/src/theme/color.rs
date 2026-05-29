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
            // Discord-inspired dark backgrounds with layered depth
            background: SerializableColor::new(30, 31, 34),        // #1e1f22
            surface: SerializableColor::new(43, 45, 49),           // #2b2d31
            surface_hover: SerializableColor::new(56, 58, 64),     // #383a40
            surface_secondary: SerializableColor::new(49, 51, 56), // #313338

            // High-contrast text hierarchy
            text: SerializableColor::new(242, 243, 245),           // #f2f3f5
            text_secondary: SerializableColor::new(181, 186, 193), // #b5bac1
            text_muted: SerializableColor::new(148, 155, 164),     // #949ba4

            // Discord blurple as primary
            primary: SerializableColor::new(88, 101, 242),      // #5865f2
            primary_hover: SerializableColor::new(71, 82, 196), // #4752c4
            secondary: SerializableColor::new(82, 136, 193),    // #5288c1 (Telegram blue)

            // Discord status colors
            success: SerializableColor::new(35, 165, 90),  // #23a55a
            danger: SerializableColor::new(218, 55, 60),   // #da373c
            warning: SerializableColor::new(240, 178, 50), // #f0b232

            victory: SerializableColor::new(35, 165, 90), // #23a55a
            defeat: SerializableColor::new(218, 55, 60),  // #da373c

            // Sidebar - deepest layer
            sidebar_bg: SerializableColor::new(30, 31, 34),          // #1e1f22
            sidebar_item_hover: SerializableColor::new(53, 55, 60),  // #35373c
            sidebar_item_active: SerializableColor::new(64, 66, 73), // #404249

            // Subtle borders
            border: SerializableColor::new(63, 65, 71),       // #3f4147
            border_light: SerializableColor::new(53, 55, 60), // #35373c

            // Gradient: blurple to Telegram blue
            gradient_start: SerializableColor::new(88, 101, 242), // #5865f2
            gradient_end: SerializableColor::new(82, 136, 193),   // #5288c1
            accent: SerializableColor::new(88, 101, 242),         // #5865f2
            accent_muted: SerializableColor::new(60, 69, 165),    // #3c45a5
        }
    }

    pub fn light() -> Self {
        Self {
            // Telegram-inspired light backgrounds
            background: SerializableColor::new(255, 255, 255),        // #ffffff
            surface: SerializableColor::new(240, 241, 243),           // #f0f1f3
            surface_hover: SerializableColor::new(232, 232, 232),     // #e8e8e8
            surface_secondary: SerializableColor::new(245, 245, 247), // #f5f5f7

            // Near-black text with clear hierarchy
            text: SerializableColor::new(26, 26, 26),           // #1a1a1a
            text_secondary: SerializableColor::new(90, 90, 90), // #5a5a5a
            text_muted: SerializableColor::new(138, 138, 138),  // #8a8a8a

            // Telegram blue as primary
            primary: SerializableColor::new(51, 144, 236),       // #3390ec
            primary_hover: SerializableColor::new(43, 127, 212), // #2b7fd4
            secondary: SerializableColor::new(88, 101, 242),     // #5865f2

            // Vibrant semantic colors
            success: SerializableColor::new(45, 165, 82),  // #2da552
            danger: SerializableColor::new(229, 57, 53),   // #e53935
            warning: SerializableColor::new(230, 162, 24), // #e6a218

            victory: SerializableColor::new(45, 165, 82), // #2da552
            defeat: SerializableColor::new(229, 57, 53),  // #e53935

            // Light sidebar
            sidebar_bg: SerializableColor::new(240, 241, 243),          // #f0f1f3
            sidebar_item_hover: SerializableColor::new(227, 228, 230),  // #e3e4e6
            sidebar_item_active: SerializableColor::new(213, 215, 218), // #d5d7da

            border: SerializableColor::new(220, 220, 220),       // #dcdcdc
            border_light: SerializableColor::new(235, 235, 235), // #ebebeb

            gradient_start: SerializableColor::new(51, 144, 236), // #3390ec
            gradient_end: SerializableColor::new(88, 101, 242),   // #5865f2
            accent: SerializableColor::new(51, 144, 236),         // #3390ec
            accent_muted: SerializableColor::new(122, 178, 236),  // #7ab2ec
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
    // Discord-inspired dark theme
    pub const DARK: Self = Self {
        background: Color32::from_rgb(30, 31, 34),        // #1e1f22
        surface: Color32::from_rgb(43, 45, 49),           // #2b2d31
        surface_hover: Color32::from_rgb(56, 58, 64),     // #383a40
        surface_secondary: Color32::from_rgb(49, 51, 56), // #313338

        text: Color32::from_rgb(242, 243, 245),           // #f2f3f5
        text_secondary: Color32::from_rgb(181, 186, 193), // #b5bac1
        text_muted: Color32::from_rgb(148, 155, 164),     // #949ba4

        primary: Color32::from_rgb(88, 101, 242),      // #5865f2
        primary_hover: Color32::from_rgb(71, 82, 196), // #4752c4
        secondary: Color32::from_rgb(82, 136, 193),    // #5288c1

        success: Color32::from_rgb(35, 165, 90),  // #23a55a
        danger: Color32::from_rgb(218, 55, 60),   // #da373c
        warning: Color32::from_rgb(240, 178, 50), // #f0b232

        victory: Color32::from_rgb(35, 165, 90), // #23a55a
        defeat: Color32::from_rgb(218, 55, 60),  // #da373c

        sidebar_bg: Color32::from_rgb(30, 31, 34),          // #1e1f22
        sidebar_item_hover: Color32::from_rgb(53, 55, 60),  // #35373c
        sidebar_item_active: Color32::from_rgb(64, 66, 73), // #404249

        border: Color32::from_rgb(63, 65, 71),       // #3f4147
        border_light: Color32::from_rgb(53, 55, 60), // #35373c

        gradient_start: Color32::from_rgb(88, 101, 242), // #5865f2
        gradient_end: Color32::from_rgb(82, 136, 193),   // #5288c1
        accent: Color32::from_rgb(88, 101, 242),         // #5865f2
        accent_muted: Color32::from_rgb(60, 69, 165),    // #3c45a5
    };

    // Telegram-inspired light theme
    pub const LIGHT: Self = Self {
        background: Color32::WHITE,                          // #ffffff
        surface: Color32::from_rgb(240, 241, 243),           // #f0f1f3
        surface_hover: Color32::from_rgb(232, 232, 232),     // #e8e8e8
        surface_secondary: Color32::from_rgb(245, 245, 247), // #f5f5f7

        text: Color32::from_rgb(26, 26, 26),           // #1a1a1a
        text_secondary: Color32::from_rgb(90, 90, 90), // #5a5a5a
        text_muted: Color32::from_rgb(138, 138, 138),  // #8a8a8a

        primary: Color32::from_rgb(51, 144, 236),       // #3390ec
        primary_hover: Color32::from_rgb(43, 127, 212), // #2b7fd4
        secondary: Color32::from_rgb(88, 101, 242),     // #5865f2

        success: Color32::from_rgb(45, 165, 82),  // #2da552
        danger: Color32::from_rgb(229, 57, 53),   // #e53935
        warning: Color32::from_rgb(230, 162, 24), // #e6a218

        victory: Color32::from_rgb(45, 165, 82), // #2da552
        defeat: Color32::from_rgb(229, 57, 53),  // #e53935

        sidebar_bg: Color32::from_rgb(240, 241, 243),          // #f0f1f3
        sidebar_item_hover: Color32::from_rgb(227, 228, 230),  // #e3e4e6
        sidebar_item_active: Color32::from_rgb(213, 215, 218), // #d5d7da

        border: Color32::from_rgb(220, 220, 220),       // #dcdcdc
        border_light: Color32::from_rgb(235, 235, 235), // #ebebeb

        gradient_start: Color32::from_rgb(51, 144, 236), // #3390ec
        gradient_end: Color32::from_rgb(88, 101, 242),   // #5865f2
        accent: Color32::from_rgb(51, 144, 236),         // #3390ec
        accent_muted: Color32::from_rgb(122, 178, 236),  // #7ab2ec
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
    pub const SMALL: u8 = 4;
    pub const MEDIUM: u8 = 8;
    pub const LARGE: u8 = 12;
    pub const ROUND: u8 = 255;
}

pub mod font_size {
    pub const SMALL: f32 = 12.0;
    pub const BODY: f32 = 14.0;
    pub const MEDIUM: f32 = 16.0;
    pub const LARGE: f32 = 20.0;
    pub const XLARGE: f32 = 24.0;
    pub const TITLE: f32 = 32.0;
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
