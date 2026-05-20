//! Chat-widget wiring for IDE context prompt injection.

use firefam_app_server_protocol::UserInput;

use super::ChatWidget;

#[derive(Default)]
pub(super) struct IdeContextState {
    enabled: bool,
    prompt_fetch_warned: bool,
}

impl IdeContextState {
    pub(super) fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn mark_available(&mut self) {
        self.prompt_fetch_warned = false;
    }
}

impl ChatWidget {
    /// Fetches fresh IDE context for the outgoing user turn and folds it into the prompt.
    pub(super) fn maybe_apply_ide_context(&mut self, items: &mut Vec<UserInput>) {
        if !self.ide_context.is_enabled() {
            return;
        }

        match crate::ide_context::fetch_ide_context(&self.config.cwd) {
            Ok(context) => {
                self.ide_context.mark_available();
                self.sync_ide_context_status_indicator();
                crate::ide_context::apply_ide_context_to_user_input(&context, items);
            }
            Err(err) => {
                self.sync_ide_context_status_indicator();
                if !self.ide_context.prompt_fetch_warned {
                    self.ide_context.prompt_fetch_warned = true;
                    self.add_info_message(
                        "IDE context was skipped for this message.".to_string(),
                        Some(err.prompt_skip_hint()),
                    );
                }
            }
        }
    }

    pub(super) fn sync_ide_context_status_indicator(&mut self) {
        self.bottom_pane
            .set_ide_context_active(self.ide_context.is_enabled());
    }
}
