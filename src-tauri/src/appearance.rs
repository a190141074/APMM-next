//! 配色持久契约和边界校验；颜色只接受不透明 sRGB，不允许注入任意 CSS。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ThemePalette {
    pub canvas: String,
    pub surface: String,
    pub surface_alt: String,
    pub sidebar: String,
    pub popover: String,
    pub image_bg: String,
    pub text: String,
    pub muted: String,
    pub border: String,
    pub accent: String,
    pub on_accent: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct ThemeScheme {
    pub id: String,
    pub name: String,
    pub light: ThemePalette,
    pub dark: ThemePalette,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src-ui/core/ipc/generated/")]
pub struct AppearanceSettings {
    pub schema: u32,
    pub mode: String,
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default)]
    pub cat_animation: bool,
    pub active_scheme_id: String,
    pub schemes: Vec<ThemeScheme>,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self { schema: 1, mode: "system".into(), reduce_motion: false, cat_animation: false, active_scheme_id: "default".into(), schemes: vec![] }
    }
}

impl AppearanceSettings {
    /// 整批校验后才能提交；非法方案不能破坏已有可读配色。
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || !matches!(self.mode.as_str(), "system" | "light" | "dark") {
            return Err("配色版本或主题模式无效".into());
        }
        if self.schemes.len() > 64 { return Err("最多保存 64 套配色".into()); }
        let mut ids = std::collections::HashSet::new();
        for scheme in &self.schemes {
            if scheme.id == "default" || uuid::Uuid::parse_str(&scheme.id).is_err() || !ids.insert(&scheme.id) {
                return Err("配色标识无效或重复".into());
            }
            if scheme.name.trim().is_empty() || scheme.name.chars().count() > 80 {
                return Err("配色名称应为 1–80 个字符".into());
            }
            for palette in [&scheme.light, &scheme.dark] {
                let value = serde_json::to_value(palette).map_err(|e| e.to_string())?;
                for color in value.as_object().unwrap().values() {
                    let color = color.as_str().unwrap();
                    if color.len() != 7 || !color.starts_with('#') || !color[1..].bytes().all(|v| v.is_ascii_hexdigit()) {
                        return Err("颜色必须为 #RRGGBB 格式".into());
                    }
                }
            }
        }
        if self.active_scheme_id != "default" && !ids.contains(&self.active_scheme_id) {
            return Err("当前配色方案不存在".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_css_and_missing_selection_without_affecting_defaults() {
        let defaults: serde_json::Value = serde_json::from_str(include_str!("../../config/themes.json")).unwrap();
        let mut settings = AppearanceSettings::default();
        settings.validate().unwrap();
        let scheme = ThemeScheme { id: uuid::Uuid::new_v4().to_string(), name: "配色".into(), light: serde_json::from_value(defaults["light"].clone()).unwrap(), dark: serde_json::from_value(defaults["dark"].clone()).unwrap() };
        settings.active_scheme_id = scheme.id.clone();
        settings.schemes.push(scheme);
        settings.validate().unwrap();
        settings.schemes[0].light.accent = "url(https://invalid)".into();
        assert!(settings.validate().is_err());
        settings.schemes.clear();
        assert!(settings.validate().is_err());
    }
}
