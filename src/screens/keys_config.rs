use gpui::{AnyElement, Context, Element, ParentElement, Styled, div, rgb};

use crate::HsmApp;

impl HsmApp {
    pub fn render_keys_config_screen(&mut self, _cx: &mut Context<'_, Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x2e2e2e))
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .flex()
                    .justify_center()
                    .text_2xl()
                    .text_color(rgb(0xffffff))
                    .child("Keys config"),
            )
            .child(
                div()
                    .flex_grow()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .text_color(rgb(0xcccccc))
                    .child("keys config"),
            )
            .into_any()
    }
}
