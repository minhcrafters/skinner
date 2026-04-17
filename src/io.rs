use crate::skin::SkinTexture;
use std::path::Path;

pub fn load_skin(path: &Path) -> Result<SkinTexture, String> {
    let img = image::open(path).map_err(|e| format!("Failed to open image: {e}"))?;
    let (w, h) = img.dimensions();

    if w != 64 || (h != 64 && h != 32) {
        return Err(format!(
            "Invalid skin dimensions: {w}×{h}. Expected 64×64 or 64×32."
        ));
    }

    let rgba = img.to_rgba8();
    let pixels: Vec<u8> = rgba.into_raw();

    // If legacy 64×32, expand to 64×64 with empty bottom half
    if h == 32 {
        let mut expanded = vec![0u8; 64 * 64 * 4];
        expanded[..pixels.len()].copy_from_slice(&pixels);
        Ok(SkinTexture::from_rgba(&expanded, 64, 64))
    } else {
        Ok(SkinTexture::from_rgba(&pixels, 64, 64))
    }
}

pub fn save_skin(path: &Path, skin: &SkinTexture) -> Result<(), String> {
    let bytes = skin.pixels_as_bytes();
    let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(skin.width, skin.height, bytes)
        .ok_or_else(|| "Failed to create image buffer".to_string())?;

    img.save(path)
        .map_err(|e| format!("Failed to save image: {e}"))
}

use image::GenericImageView;

pub fn open_file_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Minecraft Skin", &["png"])
        .add_filter("All Files", &["*"])
        .set_title("Open Minecraft Skin")
        .pick_file()
}

pub fn save_file_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_title("Save Minecraft Skin")
        .set_file_name("skin.png")
        .save_file()
}

pub fn open_palette_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("GIMP Palette", &["gpl"])
        .add_filter("All Files", &["*"])
        .set_title("Import Palette")
        .pick_file()
}

pub fn open_image_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif"])
        .add_filter("All Files", &["*"])
        .set_title("Open Reference Image")
        .pick_file()
}

pub fn save_palette_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("GIMP Palette", &["gpl"])
        .set_title("Export Palette")
        .set_file_name("palette.gpl")
        .save_file()
}
