use gpui::{
    AnyElement, Context, MouseButton, ParentElement, SharedString, Styled, div, prelude::*, px, rgb,
};

use crate::{HsmApp, SignText, VerifyText};

impl HsmApp {
    pub fn render_sign_verify_screen(&mut self, cx: &mut Context<'_, Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x2e2e2e))
            .size_full()
            .p_4()
            .gap_4()
            .child(
                // Title
                div()
                    .flex()
                    .justify_center()
                    .text_2xl()
                    .text_color(rgb(0xffffff))
                    .child("YubiHSM2 Sign & Verify Demo"),
            )
            .child(
                // Instructions
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child(
                        "Type in the input area below, then click Sign to sign the text, and Verify to verify the signature.",
                    ),
            )
            .child(
                // Input section
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcccccc))
                            .child("Input Text:"),
                    )
                    .child(
                        div()
                            .bg(rgb(0x1e1e1e))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded_md()
                            .p_2()
                            .min_h(px(40.))
                            .child(self.text_input.clone()),
                    ),
            )
            .child(
                // Buttons
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .bg(rgb(0x007acc))
                            .hover(|style| style.bg(rgb(0x005a9e)))
                            .rounded_md()
                            .px_4()
                            .py_2()
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .child("Sign")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, window, cx| {
                                    view.sign_text(&SignText, window, cx);
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(rgb(0x28a745))
                            .hover(|style| style.bg(rgb(0x1e7e34)))
                            .rounded_md()
                            .px_4()
                            .py_2()
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .child("Verify")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, window, cx| {
                                    view.verify_text(&VerifyText, window, cx);
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(rgb(0x6c757d))
                            .hover(|style| style.bg(rgb(0x5a6268)))
                            .rounded_md()
                            .px_4()
                            .py_2()
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .child("Clear")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.text_input.update(cx, |input, cx| {
                                        input.set_content(String::new(), cx);
                                    });
                                    view.signature = None;
                                    view.output_text =
                                        SharedString::from("Cleared. Ready to sign new text.");
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .child(
                // Output section
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_grow()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcccccc))
                            .child("Output:"),
                    )
                    .child(
                        div()
                            .bg(rgb(0x1e1e1e))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded_md()
                            .p_2()
                            .flex_grow()
                            .text_color(rgb(0x00ff00))
                            .text_sm()
                            .child(self.output_text.clone()),
                    ),
            )
            .into_any()
    }
}
