//! 可视化偏好设置 + 配置文件持久化。
//!
//! 配置文件：`~/.config/tmus/config.json`
//! ```json
//! { "viz": "bars|wave|ring", "color": "preset:0..5" | "#rrggbb" }
//! ```

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizKind {
    Bars,
    Wave,
    Ring,
}

impl VizKind {
    pub fn label(&self) -> &'static str {
        match self {
            VizKind::Bars => "频谱柱",
            VizKind::Wave => "波形",
            VizKind::Ring => "环形频谱",
        }
    }

    pub fn next(&self) -> VizKind {
        match self {
            VizKind::Bars => VizKind::Wave,
            VizKind::Wave => VizKind::Ring,
            VizKind::Ring => VizKind::Bars,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            VizKind::Bars => "bars",
            VizKind::Wave => "wave",
            VizKind::Ring => "ring",
        }
    }

    fn from_key(k: &str) -> VizKind {
        match k {
            "wave" => VizKind::Wave,
            "ring" => VizKind::Ring,
            _ => VizKind::Bars,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorPref {
    /// 内置预设配色（见 PRESETS 名称）
    Preset(usize),
    /// 自定义颜色（用作主色，自动生成明暗渐变）
    Custom { r: u8, g: u8, b: u8 },
}

pub const PRESET_COUNT: usize = 6;

pub fn preset_name(i: usize) -> &'static str {
    const NAMES: [&str; PRESET_COUNT] = [
        "光谱", "霓虹", "森绿", "火焰", "冰蓝", "云白",
    ];
    if i < PRESET_COUNT {
        NAMES[i]
    } else {
        "光谱"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub viz: VizKind,
    pub color: ColorPref,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            viz: VizKind::Bars,
            color: ColorPref::Preset(0),
        }
    }
}

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/tmus/config.json")
}

pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 {
        return None;
    }
    let ok = |c: char| c.is_ascii_hexdigit();
    if !t.chars().all(ok) {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some((r, g, b))
}

fn hex_of(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

pub fn load() -> Settings {
    let mut s = Settings::default();
    let Ok(text) = fs::read_to_string(config_path()) else {
        return s;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return s;
    };
    if let Some(viz) = v.get("viz").and_then(|x| x.as_str()) {
        s.viz = VizKind::from_key(viz);
    }
    if let Some(c) = v.get("color").and_then(|x| x.as_str()) {
        if let Some(rest) = c.strip_prefix("preset:") {
            if let Ok(i) = rest.parse::<usize>() {
                if i < PRESET_COUNT {
                    s.color = ColorPref::Preset(i);
                }
            }
        } else if let Some((r, g, b)) = parse_hex(c) {
            s.color = ColorPref::Custom { r, g, b };
        }
    }
    s
}

pub fn save(s: &Settings) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let color = match s.color {
        ColorPref::Preset(i) => format!("preset:{i}"),
        ColorPref::Custom { r, g, b } => hex_of(r, g, b),
    };
    let json = serde_json::json!({
        "viz": s.viz.key(),
        "color": color,
    });
    let _ = fs::write(path, serde_json::to_string_pretty(&json).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_works() {
        assert_eq!(parse_hex("#00ff88"), Some((0, 255, 136)));
        assert_eq!(parse_hex("ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex("#12345"), None); // 长度不对
        assert_eq!(parse_hex("#zz0000"), None); // 非法字符
    }

    #[test]
    fn save_load_roundtrip() {
        // 用临时 HOME 隔离，避免污染真实配置
        let tmp = std::env::temp_dir().join(format!("tmus-cfg-test-{}", std::process::id()));
        std::env::set_var("HOME", &tmp);

        let s = Settings {
            viz: VizKind::Ring,
            color: ColorPref::Custom { r: 0, g: 255, b: 136 },
        };
        save(&s);

        let loaded = load();
        assert_eq!(loaded.viz, VizKind::Ring);
        assert_eq!(loaded.color, ColorPref::Custom { r: 0, g: 255, b: 136 });

        let _ = fs::remove_dir_all(&tmp);
    }
}
