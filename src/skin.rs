
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkinModel {
    Classic,
    Slim,
}

#[derive(Clone)]
pub struct SkinTexture {
    pixels: Vec<[u8; 4]>,
    pub width: u32,
    pub height: u32,
    pub model: SkinModel,
    dirty: bool,
}

impl SkinTexture {
    pub fn new() -> Self {
        let mut pixels = vec![[0u8; 4]; 64 * 64];
        // Fill the head area with a default skin color so it's not invisible
        for y in 8..16 {
            for x in 8..16 {
                pixels[y * 64 + x] = [200, 170, 130, 255]; // skin tone - head front
            }
        }
        Self {
            pixels,
            width: 64,
            height: 64,
            model: SkinModel::Classic,
            dirty: true,
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize]
        } else {
            [0, 0, 0, 0]
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn pixels_as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            bytes.extend_from_slice(pixel);
        }
        bytes
    }

    pub fn from_rgba(data: &[u8], width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        let mut pixels = vec![[0u8; 4]; pixel_count];
        for i in 0..pixel_count {
            let offset = i * 4;
            if offset + 3 < data.len() {
                pixels[i] = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ];
            }
        }
        Self {
            pixels,
            width,
            height,
            model: SkinModel::Classic,
            dirty: true,
        }
    }

    pub fn to_color_image(&self) -> eframe::egui::ColorImage {
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for pixel in &self.pixels {
            rgba.extend_from_slice(pixel);
        }
        eframe::egui::ColorImage::from_rgba_unmultiplied(
            [self.width as usize, self.height as usize],
            &rgba,
        )
    }
}
