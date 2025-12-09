mod config;
mod hsm;
mod ui;

use config::*;
use gpui::{
    App, Application, Bounds, Context, Entity, EventEmitter, Focusable, IntoElement, KeyBinding,
    MouseButton, ParentElement, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
    actions, div, prelude::*, px, rgb, size,
};
use hsm::{HsmClient, HsmConfig};
use ui::TextArea;

actions!(hsm_demo, [SignText, VerifyText]);

struct HsmApp {
    text_input: Entity<TextArea>,
    output_text: SharedString,
    signature: Option<Vec<u8>>,
}

impl HsmApp {
    fn new(cx: &mut Context<'_, Self>) -> Self {
        let text_input = cx.new(|cx| TextArea::new(cx, "Type your text here...".to_string()));

        Self {
            text_input,
            output_text: SharedString::from("Ready. Type text and click Sign."),
            signature: None,
        }
    }

    fn sign_text(&mut self, _: &SignText, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.text_input.read(cx).content();
        if text.is_empty() {
            self.output_text = "Error: Input text is empty".into();
            cx.notify();
            return;
        }

        // Create HSM config
        let config = HsmConfig {
            auth_key_id: DEFAULT_AUTH_KEY_ID,
            auth_password: DEFAULT_AUTH_PASSWORD.to_string(),
        };

        // Connect to YubiHSM2 via USB and sign
        match HsmClient::connect(config) {
            Ok(client) => match hsm::sign(&client, DEFAULT_SIGNING_KEY_ID, text.as_bytes()) {
                Ok(signature) => {
                    let sig_hex = hex::encode(&signature);
                    self.signature = Some(signature);
                    self.output_text = format!(
                            "✓ Successfully signed text\n\nInput: '{}'\n\nSignature (hex):\n{}\n\nLength: {} bytes",
                            text,
                            sig_hex,
                            self.signature.as_ref().unwrap().len()
                        ).into();
                }
                Err(e) => {
                    self.output_text = format!("Signing failed: {}\n\nMake sure key ID 0x{:x} exists in your YubiHSM2 (secp256r1/ECDSA type)", e, DEFAULT_SIGNING_KEY_ID).into();
                }
            },
            Err(e) => {
                self.output_text = format!("Failed to connect to YubiHSM2 via USB: {}\n\nMake sure your YubiHSM2 is connected via USB.", e).into();
            }
        }

        cx.notify();
    }

    fn verify_text(&mut self, _: &VerifyText, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.text_input.read(cx).content();

        if text.is_empty() {
            self.output_text = "Error: Input text is empty".into();
            cx.notify();
            return;
        }

        if self.signature.is_none() {
            self.output_text = "Error: No signature to verify. Sign text first.".into();
            cx.notify();
            return;
        }

        // Create HSM config
        let config = HsmConfig {
            auth_key_id: DEFAULT_AUTH_KEY_ID,
            auth_password: DEFAULT_AUTH_PASSWORD.to_string(),
        };

        // Connect to YubiHSM2 via USB and verify
        match HsmClient::connect(config) {
            Ok(client) => {
                match hsm::verify(
                    &client,
                    DEFAULT_SIGNING_KEY_ID,
                    text.as_bytes(),
                    self.signature.as_ref().unwrap(),
                ) {
                    Ok(is_valid) => {
                        if is_valid {
                            self.output_text = format!(
                                "✓ Signature verification SUCCESSFUL\n\nInput: '{}'\n\nThe signature is valid!",
                                text
                            ).into();
                        } else {
                            self.output_text = format!(
                                "✗ Signature verification FAILED\n\nInput: '{}'\n\nThe signature does not match the text.",
                                text
                            ).into();
                        }
                    }
                    Err(e) => {
                        self.output_text = format!("Verification failed: {}", e).into();
                    }
                }
            }
            Err(e) => {
                self.output_text = format!("Failed to connect to YubiHSM2 via USB: {}", e).into();
            }
        }

        cx.notify();
    }
}

impl Render for HsmApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
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
                    .child("YubiHSM2 Sign & Verify Demo")
            )
            .child(
                // Instructions
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Type in the input area below, then click Sign to sign the text, and Verify to verify the signature.")
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
                            .child("Input Text:")
                    )
                    .child(
                        div()
                            .bg(rgb(0x1e1e1e))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded_md()
                            .p_2()
                            .min_h(px(40.))
                            .child(self.text_input.clone())
                    )
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, window, cx| {
                                view.sign_text(&SignText, window, cx);
                            }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, window, cx| {
                                view.verify_text(&VerifyText, window, cx);
                            }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                view.text_input.update(cx, |input, cx| {
                                    input.set_content(String::new(), cx);
                                });
                                view.signature = None;
                                view.output_text = "Cleared. Ready to sign new text.".into();
                                cx.notify();
                            }))
                    )
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
                            .child("Output:")
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
                            .child(self.output_text.clone())
                    )
            )
    }
}

impl EventEmitter<()> for HsmApp {}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);

        // Bind keys for textarea actions
        cx.bind_keys([
            KeyBinding::new("backspace", ui::textarea::Backspace, None),
            KeyBinding::new("delete", ui::textarea::Delete, None),
            KeyBinding::new("left", ui::textarea::Left, None),
            KeyBinding::new("right", ui::textarea::Right, None),
            KeyBinding::new("cmd-a", ui::textarea::SelectAll, None),
            KeyBinding::new("cmd-v", ui::textarea::Paste, None),
            KeyBinding::new("cmd-c", ui::textarea::Copy, None),
            KeyBinding::new("cmd-x", ui::textarea::Cut, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(HsmApp::new),
            )
            .unwrap();

        // Focus the text input on startup
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.text_input.focus_handle(cx));
            })
            .unwrap();
    });
}
