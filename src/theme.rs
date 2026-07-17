use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(74, 108, 247);
pub const LIGHT_BG: Color32 = Color32::from_rgb(247, 245, 242);
pub const LIGHT_PANEL: Color32 = Color32::from_rgb(255, 255, 255);
pub const LIGHT_TEXT: Color32 = Color32::from_rgb(26, 28, 30);
pub const DARK_BG: Color32 = Color32::from_rgb(10, 12, 16);
pub const DARK_PANEL: Color32 = Color32::from_rgb(18, 20, 24);
pub const DARK_TEXT: Color32 = Color32::from_rgb(232, 230, 225);
pub const ERROR: Color32 = Color32::from_rgb(239, 68, 68);
pub const WARNING: Color32 = Color32::from_rgb(245, 158, 11);
pub const SUCCESS: Color32 = Color32::from_rgb(34, 197, 94);
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_6: f32 = 24.0;
pub const CONTROL_HEIGHT: f32 = 32.0;

pub fn setup_fonts(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\msjh.ttc",
        r"C:\Windows\Fonts\msjhbd.ttc",
        r"C:\Windows\Fonts\mingliu.ttc",
    ];
    let Some((_, bytes)) = candidates
        .iter()
        .find_map(|path| std::fs::read(path).ok().map(|bytes| (*path, bytes)))
    else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("zh_tw".into(), egui::FontData::from_owned(bytes).into());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "zh_tw".into());
    }
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = if dark { DARK_PANEL } else { LIGHT_BG };
    visuals.window_fill = if dark { DARK_PANEL } else { LIGHT_PANEL };
    visuals.extreme_bg_color = if dark { DARK_BG } else { LIGHT_PANEL };
    visuals.override_text_color = Some(if dark { DARK_TEXT } else { LIGHT_TEXT });
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(SPACE_2, SPACE_2);
        style.spacing.button_padding = Vec2::new(SPACE_4, SPACE_2);
        style.spacing.interact_size = Vec2::new(CONTROL_HEIGHT, CONTROL_HEIGHT);
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(22.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        );
        style.animation_time = 0.14;
    });
}
