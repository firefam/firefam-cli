//! Shared popup-related constants for bottom pane widgets.

use ratatui::text::Line;

use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::keymap::ListKeymap;
use crate::keymap::primary_binding;
use crossterm::event::KeyCode;

/// Maximum number of rows any popup should attempt to display.
/// Keep this consistent across all popups for a uniform feel.
pub(crate) const MAX_POPUP_ROWS: usize = 8;

/// Standard footer hint text used by popups.
pub(crate) fn standard_popup_hint_line() -> Line<'static> {
    Line::from(vec![
        "↑/↓ Navigate · ".into(),
        key_hint::plain(KeyCode::Enter).into(),
        " Select · ".into(),
        key_hint::plain(KeyCode::Tab).into(),
        " Complete · ".into(),
        key_hint::plain(KeyCode::Esc).into(),
        " to cancel".into(),
    ])
}

pub(crate) fn standard_popup_hint_line_for_keymap(list_keymap: &ListKeymap) -> Line<'static> {
    let accept =
        primary_binding(&list_keymap.accept).unwrap_or_else(|| key_hint::plain(KeyCode::Enter));
    let cancel =
        primary_binding(&list_keymap.cancel).unwrap_or_else(|| key_hint::plain(KeyCode::Esc));
    Line::from(vec![
        "↑/↓ Navigate · ".into(),
        accept.into(),
        " Select · ".into(),
        key_hint::plain(KeyCode::Tab).into(),
        " Complete · ".into(),
        cancel.into(),
        " to cancel".into(),
    ])
}

pub(crate) fn accept_cancel_hint_line(
    accept: Option<KeyBinding>,
    accept_label: &'static str,
    cancel: Option<KeyBinding>,
    cancel_label: &'static str,
) -> Line<'static> {
    match (accept, cancel) {
        (Some(accept), Some(cancel)) => Line::from(vec![
            "Press ".into(),
            accept.into(),
            format!(" {accept_label} or ").into(),
            cancel.into(),
            format!(" {cancel_label}").into(),
        ]),
        (Some(accept), None) => Line::from(vec![
            "Press ".into(),
            accept.into(),
            format!(" {accept_label}").into(),
        ]),
        (None, Some(cancel)) => Line::from(vec![
            "Press ".into(),
            cancel.into(),
            format!(" {cancel_label}").into(),
        ]),
        (None, None) => Line::from(""),
    }
}
