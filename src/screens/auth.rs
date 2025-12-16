use gpui::{
    AnyElement, Context, Element, InteractiveElement, MouseButton, ParentElement, SharedString,
    Styled, div, rgb,
};

use crate::{HsmApp, Screen, config::DEFAULT_AUTH_KEY_ID, hsm::HsmConfig};

impl HsmApp {
    fn authenticate_session(&mut self, cx: &mut Context<'_, Self>) {
        let password = self.auth_password_input.read(cx).content();

        if password.trim().is_empty() {
            self.auth_status = SharedString::from("Password cannot be empty.");
            cx.notify();
            return;
        }

        let config = HsmConfig {
            auth_key_id: DEFAULT_AUTH_KEY_ID,
            auth_password: password,
        };

        match self.session.connect(config) {
            Ok(()) => {
                self.auth_status =
                    SharedString::from("Successfully authenticated to YubiHSM session.");
                // After successful auth, switch to main Sign & Verify screen
                self.current_screen = Screen::SignVerify;
                // Clear the password field for security
                self.auth_password_input.update(cx, |input, cx| {
                    input.set_content(String::new(), cx);
                });
            }
            Err(e) => {
                self.auth_status = format!("Authentication failed: {}", e).into();
            }
        }

        cx.notify();
    }

    pub fn render_auth_screen(&mut self, cx: &mut Context<'_, Self>) -> AnyElement {
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
                    .child("Authenticate YubiHSM session"),
            )
            .child(div().text_xs().text_color(rgb(0x888888)).child(
                "Enter the authentication password for the YubiHSM auth key, then click Connect.",
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcccccc))
                            .child(format!("Auth key ID: 0x{:04x}", DEFAULT_AUTH_KEY_ID)),
                    )
                    .child(
                        div()
                            .bg(rgb(0x1e1e1e))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded_md()
                            .p_2()
                            .min_h(gpui::px(24.))
                            // Note: TextArea doesn't mask input; this is a simple demo.
                            .child(self.auth_password_input.clone()),
                    ),
            )
            .child(
                div().flex().gap_2().child(
                    div()
                        .bg(rgb(0x28a745))
                        .hover(|style| style.bg(rgb(0x1e7e34)))
                        .rounded_md()
                        .px_4()
                        .py_2()
                        .text_color(rgb(0xffffff))
                        .cursor_pointer()
                        .child("Connect")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _, _, cx| {
                                view.authenticate_session(cx);
                            }),
                        ),
                ),
            )
            .child(
                div()
                    .bg(rgb(0x1e1e1e))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded_md()
                    .p_2()
                    .text_sm()
                    .text_color(rgb(0xcccccc))
                    .child(self.auth_status.clone()),
            )
            .into_any()
    }
}
