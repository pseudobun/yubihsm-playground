use gpui::{
    AnyElement, Context, Element, InteractiveElement, MouseButton, ParentElement, Styled, div, rgb,
};

use crate::{HsmApp, hsm};

impl HsmApp {
    fn load_keys_from_hsm(&mut self, cx: &mut Context<'_, Self>) {
        match self.session.active_client() {
            Ok(client) => match hsm::list_objects(client) {
                Ok(summary) => {
                    self.keys_output = summary.into();
                }
                Err(e) => {
                    self.keys_output =
                        format!("Failed to list objects from YubiHSM2: {}", e).into();
                }
            },
            Err(e) => {
                self.keys_output = format!(
                    "Failed to use YubiHSM2 session: {}\n\nGo to the Auth screen and authenticate first.",
                    e
                )
                .into();
            }
        }

        cx.notify();
    }

    pub fn render_keys_config_screen(&mut self, cx: &mut Context<'_, Self>) -> AnyElement {
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
            .child(div().text_xs().text_color(rgb(0x888888)).child(
                "List objects/keys that are visible to the current YubiHSM authentication key.",
            ))
            .child(
                div().flex().gap_2().child(
                    div()
                        .bg(rgb(0x007acc))
                        .hover(|style| style.bg(rgb(0x005a9e)))
                        .rounded_md()
                        .px_4()
                        .py_2()
                        .text_color(rgb(0xffffff))
                        .cursor_pointer()
                        .child("List keys")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _, _, cx| {
                                view.load_keys_from_hsm(cx);
                            }),
                        ),
                ),
            )
            .child(
                div()
                    .flex_grow()
                    .bg(rgb(0x1e1e1e))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded_md()
                    .p_2()
                    .text_color(rgb(0xcccccc))
                    .text_sm()
                    .child(self.keys_output.clone()),
            )
            .into_any()
    }
}
