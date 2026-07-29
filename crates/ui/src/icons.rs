pub const DEFAULT_ICON_SIZE: f32 = 16.0;

pub mod nav {
    pub const DASHBOARD: &str = egui_phosphor::regular::HOUSE;
    pub const FRIENDS: &str = egui_phosphor::regular::USERS;
    pub const FOLLOWS: &str = egui_phosphor::regular::STAR;
    pub const MATCHES: &str = egui_phosphor::regular::SWORD;
    pub const HEROES: &str = egui_phosphor::regular::SHIELD_STAR;
    pub const ITEMS: &str = egui_phosphor::regular::CUBE;
    pub const SETTINGS: &str = egui_phosphor::regular::GEAR;
}

pub mod action {
    pub const COLLAPSE: &str = egui_phosphor::regular::CARET_LEFT;
    pub const EXPAND: &str = egui_phosphor::regular::CARET_RIGHT;
    pub const CARET_DOWN: &str = egui_phosphor::regular::CARET_DOWN;
    pub const CLOSE: &str = egui_phosphor::regular::X;
    pub const SEARCH: &str = egui_phosphor::regular::MAGNIFYING_GLASS;
    pub const ADD: &str = egui_phosphor::regular::PLUS;
    pub const REMOVE: &str = egui_phosphor::regular::MINUS;
    pub const REFRESH: &str = egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE;
    pub const SYNC: &str = egui_phosphor::regular::ARROWS_CLOCKWISE;
    pub const MENU: &str = egui_phosphor::regular::LIST;
    pub const MORE: &str = egui_phosphor::regular::DOTS_THREE;
    pub const CHECK: &str = egui_phosphor::regular::CHECK;
    pub const WARNING: &str = egui_phosphor::regular::WARNING;
    pub const INFO: &str = egui_phosphor::regular::INFO;
    pub const EDIT: &str = egui_phosphor::regular::PENCIL_SIMPLE;
    pub const DELETE: &str = egui_phosphor::regular::TRASH;
    pub const TRACK: &str = egui_phosphor::regular::BELL;
    pub const TRACKING: &str = egui_phosphor::regular::BELL_RINGING;
    pub const ARROW_LEFT: &str = egui_phosphor::regular::ARROW_LEFT;
    pub const PANEL_LEFT: &str = egui_phosphor::regular::SIDEBAR_SIMPLE;
}

pub mod status {
    pub const VICTORY: &str = egui_phosphor::regular::TROPHY;
    pub const DEFEAT: &str = egui_phosphor::regular::X;
    pub const ONLINE: &str = egui_phosphor::regular::CIRCLE;
    pub const OFFLINE: &str = egui_phosphor::regular::CIRCLE_DASHED;
    pub const IN_GAME: &str = egui_phosphor::regular::GAME_CONTROLLER;
}

pub mod game {
    pub const STRENGTH: &str = egui_phosphor::regular::DIAMOND;
    pub const AGILITY: &str = egui_phosphor::regular::SPADE;
    pub const INTELLIGENCE: &str = egui_phosphor::regular::CLUB;
    pub const UNIVERSAL: &str = egui_phosphor::regular::STAR;
    pub const GOLD: &str = egui_phosphor::regular::COINS;
}

#[cfg(test)]
mod tests {
    #[test]
    fn menu() {
        let menu = super::action::MENU;
        println!("{menu}");
    }
}
