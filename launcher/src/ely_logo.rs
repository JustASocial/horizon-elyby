use gpui::{div, px, rgb, white, Div, FontWeight, ParentElement, Styled};

pub(super) fn logo(size: f32) -> Div {
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.28))
        .bg(rgb(0x3157b7))
        .font_family(horizon_theme::glass::FONT)
        .font_weight(FontWeight::BOLD)
        .text_size(px(size * 0.68))
        .line_height(px(size))
        .text_color(white())
        .child("e")
}
