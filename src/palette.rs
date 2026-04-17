/// Color palette management with built-in Minecraft-appropriate colors,
/// custom color support, and GIMP palette (.gpl) import/export.
use std::path::Path;

pub struct Palette {
    pub colors: Vec<[u8; 4]>,
    pub recent: Vec<[u8; 4]>,
    pub name: String,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            colors: default_palette(),
            recent: Vec::new(),
            name: "Default".to_string(),
        }
    }

    /// Add the current primary color to the palette
    pub fn add_color(&mut self, color: [u8; 4]) {
        if !self.colors.contains(&color) {
            self.colors.push(color);
        }
    }

    /// Remove a color at the given index
    pub fn remove_color(&mut self, index: usize) {
        if index < self.colors.len() {
            self.colors.remove(index);
        }
    }

    /// Reset to default palette
    pub fn reset(&mut self) {
        self.colors = default_palette();
        self.name = "Default".to_string();
    }

    /// Export palette in GIMP .gpl format
    pub fn export_gpl(&self) -> String {
        let mut out = String::new();
        out.push_str("GIMP Palette\n");
        out.push_str(&format!("Name: {}\n", self.name));
        out.push_str(&format!("Columns: 6\n"));
        out.push_str("#\n");
        for color in &self.colors {
            // GIMP palettes use RGB only (no alpha), but we store alpha in the name
            let alpha_note = if color[3] < 255 {
                format!(" (A={})", color[3])
            } else {
                String::new()
            };
            out.push_str(&format!(
                "{:>3} {:>3} {:>3}\t#{:02X}{:02X}{:02X}{}\n",
                color[0], color[1], color[2], color[0], color[1], color[2], alpha_note
            ));
        }
        out
    }

    /// Import palette from GIMP .gpl format
    pub fn import_gpl(content: &str) -> Result<Self, String> {
        let mut colors = Vec::new();
        let mut name = "Imported".to_string();
        let mut in_header = true;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Must start with "GIMP Palette"
            if in_header {
                if line == "GIMP Palette" {
                    continue;
                }
                if let Some(n) = line.strip_prefix("Name:") {
                    name = n.trim().to_string();
                    continue;
                }
                if line.starts_with("Columns:") {
                    continue;
                }
                if line.starts_with('#') {
                    in_header = false;
                    continue;
                }
            }

            // Skip comment lines
            if line.starts_with('#') {
                continue;
            }

            // Parse color line: "R G B\tOptionalName"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let r = parts[0]
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid red value: {}", parts[0]))?;
                let g = parts[1]
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid green value: {}", parts[1]))?;
                let b = parts[2]
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid blue value: {}", parts[2]))?;

                // Check if there's an alpha annotation like "(A=128)"
                let mut a = 255u8;
                let rest = parts[3..].join(" ");
                if let Some(alpha_start) = rest.find("(A=") {
                    if let Some(alpha_end) = rest[alpha_start..].find(')') {
                        let alpha_str = &rest[alpha_start + 3..alpha_start + alpha_end];
                        if let Ok(alpha_val) = alpha_str.parse::<u8>() {
                            a = alpha_val;
                        }
                    }
                }

                colors.push([r, g, b, a]);
            }
        }

        if colors.is_empty() {
            return Err("No colors found in palette file".to_string());
        }

        Ok(Self {
            colors,
            recent: Vec::new(),
            name,
        })
    }

    /// Load from a .gpl file
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read palette file: {e}"))?;
        Self::import_gpl(&content)
    }

    /// Save to a .gpl file
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let content = self.export_gpl();
        std::fs::write(path, content).map_err(|e| format!("Failed to write palette file: {e}"))
    }
}

fn default_palette() -> Vec<[u8; 4]> {
    vec![
        // Skin tones
        [255, 220, 185, 255],
        [230, 190, 150, 255],
        [200, 160, 120, 255],
        [170, 120, 80, 255],
        [140, 90, 60, 255],
        [100, 60, 40, 255],
        [60, 35, 20, 255],
        // Hair colors
        [50, 30, 15, 255],
        [80, 50, 25, 255],
        [140, 100, 50, 255],
        [200, 170, 100, 255],
        [240, 220, 150, 255],
        [180, 50, 20, 255],
        [220, 120, 40, 255],
        // Standard colors
        [255, 255, 255, 255], // white
        [200, 200, 200, 255], // light gray
        [128, 128, 128, 255], // gray
        [80, 80, 80, 255],    // dark gray
        [40, 40, 40, 255],    // near black
        [0, 0, 0, 255],       // black
        // Reds
        [255, 0, 0, 255],
        [200, 0, 0, 255],
        [150, 0, 0, 255],
        [100, 0, 0, 255],
        // Greens
        [0, 255, 0, 255],
        [0, 200, 0, 255],
        [0, 150, 0, 255],
        [0, 100, 0, 255],
        // Blues
        [0, 0, 255, 255],
        [0, 0, 200, 255],
        [0, 0, 150, 255],
        [0, 0, 100, 255],
        // Warm palette
        [255, 200, 0, 255],
        [255, 150, 0, 255],
        [255, 100, 0, 255],
        [255, 50, 0, 255],
        // Cool palette
        [0, 200, 255, 255],
        [0, 150, 200, 255],
        [100, 0, 255, 255],
        [200, 0, 255, 255],
        // Minecraft-specific
        [56, 77, 24, 255],    // grass green
        [110, 83, 56, 255],   // dirt brown
        [120, 120, 120, 255], // stone gray
        [60, 130, 200, 255],  // diamond blue
        [200, 180, 50, 255],  // gold
        [40, 40, 40, 255],    // obsidian
        [180, 180, 180, 255], // iron
        [220, 50, 50, 255],   // redstone
        // Transparent
        [0, 0, 0, 0],
    ]
}
