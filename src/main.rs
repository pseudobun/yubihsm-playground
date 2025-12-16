mod config;
mod hsm;
mod screens;
mod ui;

use config::*;
use gpui::{
    App, Application, Bounds, Context, Entity, EventEmitter, Focusable, IntoElement, KeyBinding,
    MouseButton, ParentElement, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
    actions, div, prelude::*, px, rgb, size,
};
use gpui_component::table::TableState;
use hsm::{HsmClient, HsmConfig, SessionManager};
use screens::keys_config::KeysTableDelegate;
use ui::TextArea;

actions!(hsm_demo, [SignText, VerifyText]);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Auth,
    SignVerify,
    KeysConfig,
}

pub struct HsmApp {
    auth_password_input: Entity<TextArea>,
    auth_status: SharedString,
    session: SessionManager,
    text_input: Entity<TextArea>,
    output_text: SharedString,
    signature: Option<Vec<u8>>,
    current_screen: Screen,
    keys_output: SharedString,
    keys_table: Option<Entity<TableState<KeysTableDelegate>>>,
    /// Cached keys data for deletion operations
    keys_data: Vec<hsm::ObjectSummary>,
    /// Currently selected key row index for deletion
    selected_key_row: Option<usize>,
}

impl HsmApp {
    fn new(cx: &mut Context<'_, Self>) -> Self {
        let auth_password_input =
            cx.new(|cx| TextArea::new(cx, "Enter YubiHSM auth password...".to_string()));
        let text_input = cx.new(|cx| TextArea::new(cx, "Type your text here...".to_string()));

        Self {
            auth_password_input,
            auth_status: SharedString::from("Please authenticate to the YubiHSM session."),
            session: SessionManager::new(),
            text_input,
            output_text: SharedString::from("Ready. Type text and click Sign."),
            signature: None,
            current_screen: Screen::Auth,
            keys_output: SharedString::from(
                "Click \"List keys\" to query objects from the YubiHSM2.",
            ),
            keys_table: None,
            keys_data: Vec::new(),
            selected_key_row: None,
        }
    }

    fn sign_text(&mut self, _: &SignText, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.text_input.read(cx).content();
        if text.is_empty() {
            self.output_text = "Error: Input text is empty".into();
            cx.notify();
            return;
        }

        // Use the active HSM session to sign
        match self.session.active_client() {
            Ok(client) => match hsm::sign(client, DEFAULT_SIGNING_KEY_ID, text.as_bytes()) {
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
                self.output_text = format!(
                    "Failed to use YubiHSM2 session: {}\n\nGo to the Auth screen and authenticate first.",
                    e
                )
                .into();
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

        // Use the active HSM session to verify
        match self.session.active_client() {
            Ok(client) => {
                match hsm::verify(
                    client,
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
                self.output_text = format!(
                    "Failed to use YubiHSM2 session: {}\n\nGo to the Auth screen and authenticate first.",
                    e
                )
                .into();
            }
        }

        cx.notify();
    }

    fn disconnect_session(&mut self, cx: &mut Context<'_, Self>) {
        // Drop the active HSM session
        self.session.disconnect();

        // Reset app state
        self.current_screen = Screen::Auth;
        self.auth_status =
            SharedString::from("Disconnected. Please authenticate to the YubiHSM session.");
        self.output_text = SharedString::from("Ready. Type text and click Sign.");
        self.keys_output =
            SharedString::from("Click \"List keys\" to query objects from the YubiHSM2.");
        self.signature = None;
        self.keys_table = None;
        self.keys_data = Vec::new();
        self.selected_key_row = None;

        // Clear password field
        self.auth_password_input
            .update(cx, |input, cx| input.set_content(String::new(), cx));

        cx.notify();
    }
}

impl Render for HsmApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // If not authenticated, show only the auth screen (no sidebar)
        if !self.session.is_authenticated() {
            return div()
                .flex()
                .flex_col()
                .bg(rgb(0x2e2e2e))
                .size_full()
                .child(self.render_auth_screen(cx));
        }

        // If authenticated, show full UI with sidebar
        div()
            .flex()
            .flex_row()
            .bg(rgb(0x2e2e2e))
            .size_full()
            .child(
                // Sidebar navigation
                div()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x252526))
                    .w(px(200.))
                    .p_4()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .text_color(rgb(0xffffff))
                            .child("Navigation"),
                    )
                    .child({
                        let is_active = self.current_screen == Screen::SignVerify;
                        let bg = if is_active {
                            rgb(0x3c3c3c)
                        } else {
                            rgb(0x2a2a2a)
                        };

                        div()
                            .bg(bg)
                            .hover(|style| style.bg(rgb(0x404040)))
                            .rounded_md()
                            .px_3()
                            .py_2()
                            .cursor_pointer()
                            .text_color(rgb(0xffffff))
                            .child("Sign & Verify")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.current_screen = Screen::SignVerify;
                                    cx.notify();
                                }),
                            )
                    })
                    .child({
                        let is_active = self.current_screen == Screen::KeysConfig;
                        let bg = if is_active {
                            rgb(0x3c3c3c)
                        } else {
                            rgb(0x2a2a2a)
                        };

                        div()
                            .bg(bg)
                            .hover(|style| style.bg(rgb(0x404040)))
                            .rounded_md()
                            .px_3()
                            .py_2()
                            .cursor_pointer()
                            .text_color(rgb(0xffffff))
                            .child("Keys config")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| {
                                    view.current_screen = Screen::KeysConfig;
                                    cx.notify();
                                }),
                            )
                    })
                    // Spacer to push the disconnect button to the bottom
                    .child(div().flex_grow())
                    // Centered disconnect button at the bottom
                    .child(
                        div().flex().justify_center().child(
                            div()
                                .bg(rgb(0x6c757d))
                                .hover(|style| style.bg(rgb(0x5a6268)))
                                .rounded_md()
                                .px_4()
                                .py_2()
                                .cursor_pointer()
                                .text_color(rgb(0xffffff))
                                .text_center()
                                .child("Disconnect")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| {
                                        view.disconnect_session(cx);
                                    }),
                                ),
                        ),
                    ),
            )
            .child(
                // Main content area
                match self.current_screen {
                    Screen::Auth => self.render_auth_screen(cx),
                    Screen::SignVerify => self.render_sign_verify_screen(cx),
                    Screen::KeysConfig => self.render_keys_config_screen(cx),
                },
            )
    }
}

impl EventEmitter<()> for HsmApp {}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Initialize gpui-component (theme, global state, etc.)
        gpui_component::init(cx);

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

        // Focus the auth password input on startup
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.auth_password_input.focus_handle(cx));
            })
            .unwrap();
    });
}
