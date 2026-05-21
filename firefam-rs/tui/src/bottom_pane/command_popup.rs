use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::WidgetRef;

use super::popup_consts::MAX_POPUP_ROWS;
use super::scroll_state::ScrollState;
use super::selection_popup_common::ColumnWidthConfig;
use super::selection_popup_common::ColumnWidthMode;
use super::selection_popup_common::GenericDisplayRow;
use super::selection_popup_common::measure_rows_height_with_col_width_mode;
use super::selection_popup_common::render_rows_with_col_width_mode;
use super::skill_popup::MentionItem;
use super::skill_popup::match_mention_item;
use super::skill_popup::sort_mention_matches;
use super::slash_commands::BuiltinCommandFlags;
use super::slash_commands::ServiceTierCommand;
use super::slash_commands::SlashCommandItem;
use super::slash_commands::commands_for_input;
use crate::render::Insets;
use crate::render::RectExt;
use crate::slash_command::SlashCommand;

// Hide alias commands in the default popup list so each unique action appears once.
// `quit` is an alias of `exit`, so we skip `quit` here.
const ALIAS_COMMANDS: &[SlashCommand] = &[SlashCommand::Quit];
const COMMAND_COLUMN_WIDTH: ColumnWidthConfig = ColumnWidthConfig::new(
    ColumnWidthMode::AutoAllRows,
    /*name_column_width*/ None,
);

/// A selectable item in the popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CommandItem {
    Builtin(SlashCommand),
    ServiceTier(ServiceTierCommand),
    Skill(MentionItem),
}

pub(crate) struct CommandPopup {
    command_filter: String,
    commands: Vec<CommandItem>,
    skills: Vec<MentionItem>,
    state: ScrollState,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CommandPopupFlags {
    pub(crate) collaboration_modes_enabled: bool,
    pub(crate) connectors_enabled: bool,
    pub(crate) plugins_command_enabled: bool,
    pub(crate) service_tier_commands_enabled: bool,
    pub(crate) goal_command_enabled: bool,
    pub(crate) personality_command_enabled: bool,
    pub(crate) realtime_conversation_enabled: bool,
    pub(crate) audio_device_selection_enabled: bool,
    pub(crate) windows_degraded_sandbox_active: bool,
    pub(crate) side_conversation_active: bool,
}

impl From<CommandPopupFlags> for BuiltinCommandFlags {
    fn from(value: CommandPopupFlags) -> Self {
        Self {
            collaboration_modes_enabled: value.collaboration_modes_enabled,
            connectors_enabled: value.connectors_enabled,
            plugins_command_enabled: value.plugins_command_enabled,
            service_tier_commands_enabled: value.service_tier_commands_enabled,
            goal_command_enabled: value.goal_command_enabled,
            personality_command_enabled: value.personality_command_enabled,
            realtime_conversation_enabled: value.realtime_conversation_enabled,
            audio_device_selection_enabled: value.audio_device_selection_enabled,
            allow_elevate_sandbox: value.windows_degraded_sandbox_active,
            side_conversation_active: value.side_conversation_active,
        }
    }
}

impl CommandPopup {
    pub(crate) fn new(
        flags: CommandPopupFlags,
        service_tier_commands: Vec<ServiceTierCommand>,
        skills: Vec<MentionItem>,
    ) -> Self {
        // Keep built-in availability in sync with the composer.
        let commands = commands_for_input(flags.into(), &service_tier_commands)
            .into_iter()
            .filter_map(|command| match command {
                SlashCommandItem::Builtin(cmd) => (!cmd.command().starts_with("debug")
                    && cmd != SlashCommand::Apps)
                    .then_some(CommandItem::Builtin(cmd)),
                SlashCommandItem::ServiceTier(command) => Some(CommandItem::ServiceTier(command)),
            })
            .collect();
        Self {
            command_filter: String::new(),
            commands,
            skills,
            state: ScrollState::new(),
        }
    }

    pub(crate) fn set_skills(&mut self, skills: Vec<MentionItem>) {
        self.skills = skills;
        let matches_len = self.filtered_items().len();
        self.state.clamp_selection(matches_len);
        self.state
            .ensure_visible(matches_len, MAX_POPUP_ROWS.min(matches_len));
    }

    /// Update the filter string based on the current composer text. The text
    /// passed in is expected to start with a leading '/'. Everything after the
    /// *first* '/' on the *first* line becomes the active filter that is used
    /// to narrow down the list of available commands.
    pub(crate) fn on_composer_text_change(&mut self, text: String) {
        let first_line = text.lines().next().unwrap_or("");

        if let Some(stripped) = first_line.strip_prefix('/') {
            // Extract the *first* token (sequence of non-whitespace
            // characters) after the slash so that `/clear something` still
            // shows the help for `/clear`.
            let token = stripped.trim_start();
            let cmd_token = token.split_whitespace().next().unwrap_or("");

            // Update the filter keeping the original case (commands are all
            // lower-case for now but this may change in the future).
            self.command_filter = cmd_token.to_string();
        } else {
            // The composer no longer starts with '/'. Reset the filter so the
            // popup shows the *full* command list if it is still displayed
            // for some reason.
            self.command_filter.clear();
        }

        // Reset or clamp selected index based on new filtered list.
        let matches_len = self.filtered_items().len();
        self.state.clamp_selection(matches_len);
        self.state
            .ensure_visible(matches_len, MAX_POPUP_ROWS.min(matches_len));
    }

    /// Determine the preferred height of the popup for a given width.
    /// Accounts for wrapped descriptions so that long tooltips don't overflow.
    pub(crate) fn calculate_required_height(&self, width: u16) -> u16 {
        let rows = self.rows_from_matches(self.filtered());

        measure_rows_height_with_col_width_mode(
            &rows,
            &self.state,
            MAX_POPUP_ROWS,
            width,
            COMMAND_COLUMN_WIDTH,
        )
    }

    /// Compute exact/prefix matches over built-in commands and user prompts,
    /// paired with optional highlight indices. Preserves the original
    /// presentation order for built-ins and prompts.
    fn filtered(&self) -> Vec<(CommandItem, Option<Vec<usize>>)> {
        let filter = self.command_filter.trim();
        let mut out: Vec<(CommandItem, Option<Vec<usize>>)> = Vec::new();
        if filter.is_empty() {
            for command in self.commands.iter() {
                if matches!(command, CommandItem::Builtin(cmd) if ALIAS_COMMANDS.contains(cmd)) {
                    continue;
                }
                out.push((command.clone(), None));
            }
            out.extend(
                self.filtered_skills(filter)
                    .into_iter()
                    .map(|(skill, indices, _score)| (CommandItem::Skill(skill), indices)),
            );
            return out;
        }

        let filter_lower = filter.to_lowercase();
        let filter_chars = filter.chars().count();
        let mut exact: Vec<(CommandItem, Option<Vec<usize>>)> = Vec::new();
        let mut prefix: Vec<(CommandItem, Option<Vec<usize>>)> = Vec::new();
        let indices_for = |offset| Some((offset..offset + filter_chars).collect());

        let mut push_match =
            |item: CommandItem, display: &str, name: Option<&str>, name_offset: usize| {
                let display_lower = display.to_lowercase();
                let name_lower = name.map(str::to_lowercase);
                let display_exact = display_lower == filter_lower;
                let name_exact = name_lower.as_deref() == Some(filter_lower.as_str());
                if display_exact || name_exact {
                    let offset = if display_exact { 0 } else { name_offset };
                    exact.push((item, indices_for(offset)));
                    return;
                }
                let display_prefix = display_lower.starts_with(&filter_lower);
                let name_prefix = name_lower
                    .as_ref()
                    .is_some_and(|name| name.starts_with(&filter_lower));
                if display_prefix || name_prefix {
                    let offset = if display_prefix { 0 } else { name_offset };
                    prefix.push((item, indices_for(offset)));
                }
            };

        for command in self.commands.iter() {
            let display = command.command();
            push_match(command.clone(), display, None, 0);
        }

        out.extend(exact);
        out.extend(prefix);
        out.extend(
            self.filtered_skills(filter)
                .into_iter()
                .map(|(skill, indices, _score)| (CommandItem::Skill(skill), indices)),
        );
        out
    }

    fn filtered_skills(&self, filter: &str) -> Vec<(MentionItem, Option<Vec<usize>>, i32)> {
        let mut matches = Vec::new();
        for (idx, skill) in self.skills.iter().enumerate() {
            if let Some((indices, score)) = match_mention_item(skill, filter) {
                matches.push((idx, indices, score));
            }
        }
        sort_mention_matches(&mut matches, &self.skills, filter);
        matches
            .into_iter()
            .map(|(idx, indices, score)| (self.skills[idx].clone(), indices, score))
            .collect()
    }

    fn filtered_items(&self) -> Vec<CommandItem> {
        self.filtered().into_iter().map(|(c, _)| c).collect()
    }

    fn rows_from_matches(
        &self,
        matches: Vec<(CommandItem, Option<Vec<usize>>)>,
    ) -> Vec<GenericDisplayRow> {
        matches
            .into_iter()
            .map(|(item, indices)| GenericDisplayRow {
                name: item.display_name(),
                name_prefix_spans: item.name_prefix_spans(),
                match_indices: item.match_indices(indices),
                display_shortcut: None,
                description: item.description(),
                category_tag: item.category_tag(),
                wrap_indent: None,
                is_disabled: false,
                disabled_reason: None,
            })
            .collect()
    }

    /// Move the selection cursor one step up.
    pub(crate) fn move_up(&mut self) {
        let len = self.filtered_items().len();
        self.state.move_up_wrap(len);
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len));
    }

    /// Move the selection cursor one step down.
    pub(crate) fn move_down(&mut self) {
        let matches_len = self.filtered_items().len();
        self.state.move_down_wrap(matches_len);
        self.state
            .ensure_visible(matches_len, MAX_POPUP_ROWS.min(matches_len));
    }

    /// Return currently selected command, if any.
    pub(crate) fn selected_item(&self) -> Option<CommandItem> {
        let matches = self.filtered_items();
        self.state
            .selected_idx
            .and_then(|idx| matches.get(idx).cloned())
    }
}

impl CommandItem {
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Builtin(cmd) => cmd.command(),
            Self::ServiceTier(command) => &command.name,
            Self::Skill(skill) => skill
                .insert_text
                .strip_prefix('$')
                .unwrap_or(&skill.insert_text),
        }
    }

    pub(crate) fn slash_command_name(&self) -> Option<&str> {
        match self {
            Self::Builtin(cmd) => Some(cmd.command()),
            Self::ServiceTier(command) => Some(&command.name),
            Self::Skill(_) => None,
        }
    }

    fn display_name(&self) -> String {
        match self {
            Self::Builtin(cmd) => format!("/{}", cmd.command()),
            Self::ServiceTier(command) => format!("/{}", command.name),
            Self::Skill(skill) => skill.display_name.clone(),
        }
    }

    fn name_prefix_spans(&self) -> Vec<ratatui::text::Span<'static>> {
        match self {
            Self::Builtin(_) | Self::ServiceTier(_) => Vec::new(),
            Self::Skill(_) => vec!["$".dim()],
        }
    }

    fn match_indices(&self, indices: Option<Vec<usize>>) -> Option<Vec<usize>> {
        match self {
            Self::Builtin(_) | Self::ServiceTier(_) => {
                indices.map(|v| v.into_iter().map(|i| i + 1).collect())
            }
            Self::Skill(_) => indices,
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            Self::Builtin(cmd) => Some(cmd.description().to_string()),
            Self::ServiceTier(command) => Some(command.description.clone()),
            Self::Skill(skill) => skill.description.clone(),
        }
    }

    fn category_tag(&self) -> Option<String> {
        match self {
            Self::Builtin(_) | Self::ServiceTier(_) => None,
            Self::Skill(_) => Some("[Skill]".to_string()),
        }
    }
}

impl WidgetRef for CommandPopup {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let rows = self.rows_from_matches(self.filtered());
        render_rows_with_col_width_mode(
            area.inset(Insets::tlbr(
                /*top*/ 0, /*left*/ 2, /*bottom*/ 0, /*right*/ 0,
            )),
            buf,
            &rows,
            &self.state,
            MAX_POPUP_ROWS,
            "no matches",
            COMMAND_COLUMN_WIDTH,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn skill_item(display_name: &str, skill_name: &str) -> MentionItem {
        MentionItem {
            display_name: display_name.to_string(),
            description: Some(format!("{display_name} skill")),
            insert_text: format!("${skill_name}"),
            search_terms: vec![skill_name.to_string(), display_name.to_string()],
            path: Some(format!("/tmp/{skill_name}/SKILL.md")),
            category_tag: Some("[Skill]".to_string()),
            sort_rank: 1,
        }
    }

    #[test]
    fn filter_includes_init_when_typing_prefix() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        // Simulate the composer line starting with '/in' so the popup filters
        // matching commands by prefix.
        popup.on_composer_text_change("/in".to_string());

        // Access the filtered list via the selected command and ensure that
        // one of the matches is the new "init" command.
        let matches = popup.filtered_items();
        let has_init = matches.iter().any(|item| match item {
            CommandItem::Builtin(cmd) => cmd.command() == "init",
            CommandItem::ServiceTier(_) => false,
            CommandItem::Skill(_) => false,
        });
        assert!(
            has_init,
            "expected '/init' to appear among filtered commands"
        );
    }

    #[test]
    fn selecting_init_by_exact_match() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/init".to_string());

        // When an exact match exists, the selected command should be that
        // command by default.
        let selected = popup.selected_item();
        match selected {
            Some(CommandItem::Builtin(cmd)) => assert_eq!(cmd.command(), "init"),
            Some(CommandItem::ServiceTier(command)) => {
                panic!("expected init command, got service tier {command:?}")
            }
            Some(CommandItem::Skill(skill)) => panic!("expected init command, got skill {skill:?}"),
            None => panic!("expected a selected command for exact match"),
        }
    }

    #[test]
    fn model_is_first_suggestion_for_mo() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/mo".to_string());
        let matches = popup.filtered_items();
        match matches.first() {
            Some(CommandItem::Builtin(cmd)) => assert_eq!(cmd.command(), "model"),
            Some(CommandItem::ServiceTier(command)) => {
                panic!("expected model command, got service tier {command:?}")
            }
            Some(CommandItem::Skill(skill)) => {
                panic!("expected model command, got skill {skill:?}")
            }
            None => panic!("expected at least one match for '/mo'"),
        }
    }

    #[test]
    fn service_tier_command_uses_catalog_name_and_description() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags {
                service_tier_commands_enabled: true,
                ..CommandPopupFlags::default()
            },
            vec![ServiceTierCommand {
                id: "priority".to_string(),
                name: "fast".to_string(),
                description: "Fastest inference with increased plan usage".to_string(),
            }],
            Vec::new(),
        );
        popup.on_composer_text_change("/fa".to_string());

        match popup.selected_item() {
            Some(CommandItem::ServiceTier(command)) => assert_eq!(
                command,
                ServiceTierCommand {
                    id: "priority".to_string(),
                    name: "fast".to_string(),
                    description: "Fastest inference with increased plan usage".to_string(),
                }
            ),
            Some(CommandItem::Skill(skill)) => {
                panic!("expected fast service tier to be selected, got skill {skill:?}")
            }
            other => panic!("expected fast service tier to be selected, got {other:?}"),
        }
        let rows = popup.rows_from_matches(popup.filtered());
        assert_eq!(
            rows.first().and_then(|row| row.description.as_deref()),
            Some("Fastest inference with increased plan usage")
        );
    }

    #[test]
    fn empty_filter_appends_skills_after_commands() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags::default(),
            Vec::new(),
            vec![
                skill_item("Zed Skill", "zed"),
                skill_item("Alpha Skill", "alpha"),
            ],
        );
        popup.on_composer_text_change("/".to_string());

        let items = popup.filtered_items();
        let first_skill_idx = items
            .iter()
            .position(|item| matches!(item, CommandItem::Skill(_)))
            .expect("expected at least one skill item");

        assert!(first_skill_idx > 0);
        assert!(
            items[..first_skill_idx]
                .iter()
                .all(|item| !matches!(item, CommandItem::Skill(_))),
            "expected skills to be appended after commands, got {items:?}"
        );

        let skill_names = items[first_skill_idx..]
            .iter()
            .map(|item| match item {
                CommandItem::Skill(skill) => skill.display_name.clone(),
                other => panic!("expected only skill items after first skill, got {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            skill_names,
            vec!["Alpha Skill".to_string(), "Zed Skill".to_string()]
        );
    }

    #[test]
    fn skill_matches_are_available_in_slash_filter() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags::default(),
            Vec::new(),
            vec![skill_item("Firefam Debugger", "firefam-debugger")],
        );
        popup.on_composer_text_change("/fire".to_string());

        match popup.selected_item() {
            Some(CommandItem::Skill(skill)) => {
                assert_eq!(skill.insert_text, "$firefam-debugger");
            }
            other => panic!("expected matching skill to be selected, got {other:?}"),
        }

        let rows = popup.rows_from_matches(popup.filtered());
        let row = rows.first().expect("expected one matching row");
        assert_eq!(row.name, "Firefam Debugger");
        assert_eq!(row.category_tag.as_deref(), Some("[Skill]"));
    }

    #[test]
    fn filtered_commands_keep_presentation_order_for_prefix() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/m".to_string());

        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();
        assert_eq!(
            cmds,
            vec![
                "model".to_string(),
                "memories".to_string(),
                "mention".to_string(),
                "mcp".to_string()
            ]
        );
    }

    #[test]
    fn prefix_filter_limits_matches_for_ac() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/ac".to_string());

        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();
        assert!(
            !cmds.iter().any(|cmd| cmd == "compact"),
            "expected prefix search for '/ac' to exclude 'compact', got {cmds:?}"
        );
    }

    #[test]
    fn quit_hidden_in_empty_filter_but_shown_for_prefix() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/".to_string());
        let items = popup.filtered_items();
        assert!(!items.contains(&CommandItem::Builtin(SlashCommand::Quit)));

        popup.on_composer_text_change("/qu".to_string());
        let items = popup.filtered_items();
        assert!(items.contains(&CommandItem::Builtin(SlashCommand::Quit)));
    }

    #[test]
    fn plan_command_hidden_when_collaboration_modes_disabled() {
        let mut popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        popup.on_composer_text_change("/".to_string());

        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();
        assert!(
            !cmds.iter().any(|cmd| cmd == "plan"),
            "expected '/plan' to be hidden when collaboration modes are disabled, got {cmds:?}"
        );
    }

    #[test]
    fn plan_command_visible_when_collaboration_modes_enabled() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags {
                collaboration_modes_enabled: true,
                connectors_enabled: false,
                plugins_command_enabled: false,
                service_tier_commands_enabled: false,
                goal_command_enabled: false,
                personality_command_enabled: true,
                realtime_conversation_enabled: false,
                audio_device_selection_enabled: false,
                windows_degraded_sandbox_active: false,
                side_conversation_active: false,
            },
            Vec::new(),
            Vec::new(),
        );
        popup.on_composer_text_change("/plan".to_string());

        match popup.selected_item() {
            Some(CommandItem::Builtin(cmd)) => assert_eq!(cmd.command(), "plan"),
            Some(CommandItem::ServiceTier(command)) => {
                panic!("expected plan command, got service tier {command:?}")
            }
            Some(CommandItem::Skill(skill)) => panic!("expected plan command, got skill {skill:?}"),
            other => panic!("expected plan to be selected for exact match, got {other:?}"),
        }
    }

    #[test]
    fn personality_command_hidden_when_disabled() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags {
                collaboration_modes_enabled: true,
                connectors_enabled: false,
                plugins_command_enabled: false,
                service_tier_commands_enabled: false,
                goal_command_enabled: false,
                personality_command_enabled: false,
                realtime_conversation_enabled: false,
                audio_device_selection_enabled: false,
                windows_degraded_sandbox_active: false,
                side_conversation_active: false,
            },
            Vec::new(),
            Vec::new(),
        );
        popup.on_composer_text_change("/pers".to_string());

        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();
        assert!(
            !cmds.iter().any(|cmd| cmd == "personality"),
            "expected '/personality' to be hidden when disabled, got {cmds:?}"
        );
    }

    #[test]
    fn personality_command_visible_when_enabled() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags {
                collaboration_modes_enabled: true,
                connectors_enabled: false,
                plugins_command_enabled: false,
                service_tier_commands_enabled: false,
                goal_command_enabled: false,
                personality_command_enabled: true,
                realtime_conversation_enabled: false,
                audio_device_selection_enabled: false,
                windows_degraded_sandbox_active: false,
                side_conversation_active: false,
            },
            Vec::new(),
            Vec::new(),
        );
        popup.on_composer_text_change("/personality".to_string());

        match popup.selected_item() {
            Some(CommandItem::Builtin(cmd)) => assert_eq!(cmd.command(), "personality"),
            Some(CommandItem::ServiceTier(command)) => {
                panic!("expected personality command, got service tier {command:?}")
            }
            Some(CommandItem::Skill(skill)) => {
                panic!("expected personality command, got skill {skill:?}")
            }
            other => panic!("expected personality to be selected for exact match, got {other:?}"),
        }
    }

    #[test]
    fn settings_command_hidden_when_audio_device_selection_is_disabled() {
        let mut popup = CommandPopup::new(
            CommandPopupFlags {
                collaboration_modes_enabled: false,
                connectors_enabled: false,
                plugins_command_enabled: false,
                service_tier_commands_enabled: false,
                goal_command_enabled: false,
                personality_command_enabled: true,
                realtime_conversation_enabled: true,
                audio_device_selection_enabled: false,
                windows_degraded_sandbox_active: false,
                side_conversation_active: false,
            },
            Vec::new(),
            Vec::new(),
        );
        popup.on_composer_text_change("/aud".to_string());

        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();

        assert!(
            !cmds.iter().any(|cmd| cmd == "settings"),
            "expected '/settings' to be hidden when audio device selection is disabled, got {cmds:?}"
        );
    }

    #[test]
    fn debug_commands_are_hidden_from_popup() {
        let popup = CommandPopup::new(CommandPopupFlags::default(), Vec::new(), Vec::new());
        let cmds: Vec<String> = popup
            .filtered_items()
            .into_iter()
            .map(|item| match item {
                CommandItem::Builtin(cmd) => cmd.command().to_string(),
                CommandItem::ServiceTier(command) => command.name,
                CommandItem::Skill(skill) => skill.display_name,
            })
            .collect();

        assert!(
            !cmds.iter().any(|name| name.starts_with("debug")),
            "expected no /debug* command in popup menu, got {cmds:?}"
        );
    }
}
