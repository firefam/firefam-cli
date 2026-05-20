use super::*;

pub(super) fn environment_selection_error_message(err: FirefamErr) -> String {
    match err {
        FirefamErr::InvalidRequest(message) => message,
        err => err.to_string(),
    }
}
