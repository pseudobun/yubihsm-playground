use gpui::{
    AnyElement, App, AppContext, Context, Element, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Styled, Window, div, prelude::FluentBuilder, px, rgb,
};
use gpui_component::table::{Column, Table, TableDelegate, TableEvent, TableState};
use yubihsm::object::Type;

use crate::{HsmApp, hsm};

/// Table delegate for displaying HSM objects in the Keys config screen.
pub struct KeysTableDelegate {
    rows: Vec<hsm::ObjectSummary>,
    columns: Vec<Column>,
}

impl KeysTableDelegate {
    pub fn new(rows: Vec<hsm::ObjectSummary>) -> Self {
        Self {
            rows,
            columns: vec![
                Column::new("id", "ID").width(80.),
                Column::new("ty", "Type").width(110.),
                Column::new("alg", "Algorithm").width(140.),
                Column::new("label", "Label").width(200.),
                Column::new("seq", "Seq").width(60.),
                Column::new("pk", "Public key (hex)").width(260.),
            ],
        }
    }
}

impl TableDelegate for KeysTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = &self.rows[row_ix];
        let col = &self.columns[col_ix];

        let text = match col.key.as_ref() {
            "id" => format!("0x{:04x}", row.object_id),
            "ty" => format!("{:?}", row.object_type),
            "alg" => format!("{:?}", row.algorithm),
            "label" => format!("{:?}", row.label),
            "seq" => format!("{}", row.sequence),
            "pk" => row
                .public_key_hex
                .as_ref()
                .map(|pk| {
                    let preview_len = pk.len().min(32);
                    format!(
                        "{}{}",
                        &pk[..preview_len],
                        if pk.len() > preview_len { "…" } else { "" }
                    )
                })
                .unwrap_or_else(|| "-".to_string()),
            _ => String::new(),
        };

        div().text_color(rgb(0xffffff)).child(text)
    }
}

impl HsmApp {
    fn load_keys_from_hsm(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.selected_key_row = None;

        match self.session.active_client() {
            Ok(client) => match hsm::list_object_summaries(client) {
                Ok(rows) => {
                    let count = rows.len();
                    self.keys_data = rows.clone();
                    let state = cx.new(|cx| {
                        TableState::new(KeysTableDelegate::new(rows), window, cx)
                            .row_selectable(true)
                    });

                    // Subscribe to table events for row selection
                    cx.subscribe_in(&state, window, |view, _table, event, _window, cx| {
                        if let TableEvent::SelectRow(row_ix) = event {
                            view.selected_key_row = Some(*row_ix);
                            cx.notify();
                        }
                    })
                    .detach();

                    self.keys_table = Some(state);
                    self.keys_output = format!(
                        "Found {} object(s) visible to the current authentication key.\nClick a row to select, then use Delete button (auth keys cannot be deleted).",
                        count
                    )
                    .into();
                }
                Err(e) => {
                    self.keys_table = None;
                    self.keys_data = Vec::new();
                    self.keys_output =
                        format!("Failed to list objects from YubiHSM2: {}", e).into();
                }
            },
            Err(e) => {
                self.keys_table = None;
                self.keys_data = Vec::new();
                self.keys_output = format!(
                    "Failed to use YubiHSM2 session: {}\n\nGo to the Auth screen and authenticate first.",
                    e
                )
                .into();
            }
        }

        cx.notify();
    }

    fn delete_selected_key(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(row_ix) = self.selected_key_row else {
            self.keys_output = "No key selected for deletion.".into();
            cx.notify();
            return;
        };

        let Some(key) = self.keys_data.get(row_ix) else {
            self.keys_output = "Selected key no longer exists.".into();
            cx.notify();
            return;
        };

        // Don't allow deleting authentication keys
        if key.object_type == Type::AuthenticationKey {
            self.keys_output = "Cannot delete authentication keys for safety reasons.".into();
            cx.notify();
            return;
        }

        let object_id = key.object_id;
        let object_type = key.object_type;

        match self.session.active_client() {
            Ok(client) => match hsm::delete_object(client, object_id, object_type) {
                Ok(()) => {
                    self.keys_output = format!(
                        "Successfully deleted object 0x{:04x} ({:?}).",
                        object_id, object_type
                    )
                    .into();
                    // Refresh the list
                    self.load_keys_from_hsm(window, cx);
                }
                Err(e) => {
                    self.keys_output = format!("Failed to delete object: {}", e).into();
                    cx.notify();
                }
            },
            Err(e) => {
                self.keys_output = format!("Failed to access HSM session: {}", e).into();
                cx.notify();
            }
        }
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
            .child({
                let can_delete = self.selected_key_row.is_some()
                    && self
                        .selected_key_row
                        .and_then(|ix| self.keys_data.get(ix))
                        .map(|k| k.object_type != Type::AuthenticationKey)
                        .unwrap_or(false);

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
                            .child("List keys")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, window, cx| {
                                    view.load_keys_from_hsm(window, cx);
                                }),
                            ),
                    )
                    .child(
                        div()
                            .bg(if can_delete {
                                rgb(0xdc3545)
                            } else {
                                rgb(0x555555)
                            })
                            .when(can_delete, |el| el.hover(|style| style.bg(rgb(0xc82333))))
                            .rounded_md()
                            .px_4()
                            .py_2()
                            .text_color(rgb(0xffffff))
                            .cursor(if can_delete {
                                gpui::CursorStyle::PointingHand
                            } else {
                                gpui::CursorStyle::Arrow
                            })
                            .child("Delete selected")
                            .when(can_delete, |el| {
                                el.on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _, window, cx| {
                                        view.delete_selected_key(window, cx);
                                    }),
                                )
                            }),
                    )
            })
            // Status / summary text
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xcccccc))
                    .child(self.keys_output.clone()),
            )
            .child({
                if let Some(ref state) = self.keys_table {
                    div()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .bg(rgb(0x1e1e1e))
                        .border_1()
                        .border_color(rgb(0x444444))
                        .rounded_md()
                        .child(
                            Table::new(state)
                                .stripe(true)
                                .bordered(true)
                                .scrollbar_visible(true, true),
                        )
                } else {
                    div()
                        .flex_1()
                        .bg(rgb(0x1e1e1e))
                        .border_1()
                        .border_color(rgb(0x444444))
                        .rounded_md()
                        .p_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("No key data loaded yet."),
                        )
                }
            })
            .into_any()
    }
}
