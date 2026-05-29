#![allow(clippy::expect_used, reason = "build script — panicking on failure is the correct behavior")]

fn main() {
    #[cfg(feature = "cjk")]
    download_cjk_font();
}

#[cfg(feature = "cjk")]
fn download_cjk_font() {
    use std::path::Path;

    const FONT_URL: &str = "https://github.com/google/fonts/raw/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf";
    const FONT_PATH: &str = "assets/fonts/NotoSansSC-Regular.ttf";

    let font_path = Path::new(FONT_PATH);

    println!("cargo::rerun-if-changed={FONT_PATH}");

    if font_path.exists() {
        return;
    }

    if let Some(parent) = font_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create font directory");
    }

    eprintln!("Downloading NotoSansSC font...");

    let bytes = reqwest::blocking::get(FONT_URL)
        .expect("failed to download NotoSansSC font")
        .bytes()
        .expect("failed to read font data");

    std::fs::write(font_path, &bytes).expect("failed to write font file");

    eprintln!("Downloaded NotoSansSC font ({} bytes)", bytes.len());
}
