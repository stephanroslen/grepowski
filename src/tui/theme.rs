use std::str::FromStr;
use ratatui::style::Color;
pub use syntect::highlighting::{Theme as SyntectTheme, Color as SyntectColor};
use syntect::highlighting::{ScopeSelectors, StyleModifier, ThemeItem, ThemeSettings};
use tachyonfx::ToRgbComponents;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub title: Color,
    pub highlight: Color,
    pub text: Color,
    pub gauge: Color,
    pub border: Color,
    pub background: Color,
}

impl Theme {
    pub fn synthwave() -> Self {
        Self {
            title: Color::Rgb(0xf8, 0x61, 0xb4),
            highlight: Color::Rgb(0x00, 0xd3, 0xbb),
            text: Color::Rgb(0xa1, 0xb1, 0xff),
            gauge: Color::Rgb(0x50, 0x03, 0x23),
            border: Color::Rgb(0x42, 0x2a, 0xd5),
            background: Color::Rgb(0x09, 0x00, 0x2f),
        }
    }
}

fn color_to_syntect(value: Color) -> SyntectColor {
    let (r, g, b) = value.to_rgb();
    SyntectColor { r, g, b, a: 0xff }
}

impl From<Theme> for SyntectTheme {
    fn from(value: Theme) -> SyntectTheme {
        let background_color = color_to_syntect(value.background);
        let text_color = color_to_syntect(value.text);
        let highlight_color = color_to_syntect(value.highlight);
        let theme = syntect::highlighting::Theme {
            name: Some("two-color".to_string()),
            settings: ThemeSettings {
                background: Some(background_color),
                foreground: Some(text_color),
                caret: None,
                selection: None,
                inactive_selection: None,
                inactive_selection_foreground: None,
                selection_border: None,
                selection_foreground: None,
                line_highlight: None,
                misspelling: None,
                gutter: None,
                gutter_foreground: Some(text_color),
                guide: None,
                active_guide: None,
                stack_guide: None,
                highlight: None,
                find_highlight: None,
                find_highlight_foreground: None,
                brackets_foreground: None,
                brackets_background: None,
                brackets_options: Default::default(),
                bracket_contents_foreground: None,
                bracket_contents_options: Default::default(),
                tags_options: Default::default(),
                tags_foreground: None,
                shadow: None,
                minimap_border: None,
                accent: None,
                popup_css: None,
                phantom_css: None,
            },
            scopes: vec![ThemeItem {
                scope: ScopeSelectors::from_str("variable, variable.other, variable.readwrite, entity.name, entity.name.type, entity.name.class, entity.name.function, entity.name.method, entity.other.inherited-class, support.type, support.class, constant, constant.numeric, constant.character, string, string.quoted, string.other").expect("Scope selector from string expected"),
                style: StyleModifier {
                    foreground: Some(highlight_color),
                    background: None,
                    font_style: None,
                },
            }],
            author: Some("auto-generated".to_string()),
        };
        theme
    }
}




