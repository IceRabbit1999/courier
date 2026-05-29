pub fn load_locale(locale: &str) -> String {
    match locale {
        "en" => include_str!("../locales/en.ftl").to_string(),
        "zh-CN" => include_str!("../locales/zh-CN.ftl").to_string(),
        _ => include_str!("../locales/en.ftl").to_string(),
    }
}
