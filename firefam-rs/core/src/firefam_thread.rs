use crate::agent::AgentStatus;
use crate::config::ConstraintResult;
use crate::goals::ExternalGoalSet;
use crate::goals::GoalRuntimeEvent;
use crate::session::Firefam;
use crate::session::SessionSettingsUpdate;
use crate::session::SteerInputError;
use firefam_features::Feature;
use firefam_otel::SessionTelemetry;
use firefam_protocol::config_types::ApprovalsReviewer;
use firefam_protocol::config_types::CollaborationMode;
use firefam_protocol::config_types::Personality;
use firefam_protocol::config_types::ReasoningSummary;
use firefam_protocol::config_types::WindowsSandboxLevel;
use firefam_protocol::error::FirefamErr;
use firefam_protocol::error::Result as FirefamResult;
use firefam_protocol::firefamai_models::ReasoningEffort;
use firefam_protocol::mcp::CallToolResult;
use firefam_protocol::models::ActivePermissionProfile;
use firefam_protocol::models::ContentItem;
use firefam_protocol::models::PermissionProfile;
use firefam_protocol::models::ResponseInputItem;
use firefam_protocol::models::ResponseItem;
use firefam_protocol::protocol::AskForApproval;
use firefam_protocol::protocol::Event;
use firefam_protocol::protocol::Op;
use firefam_protocol::protocol::SandboxPolicy;
use firefam_protocol::protocol::SessionConfiguredEvent;
use firefam_protocol::protocol::SessionSource;
use firefam_protocol::protocol::Submission;
use firefam_protocol::protocol::ThreadMemoryMode;
use firefam_protocol::protocol::ThreadSource;
use firefam_protocol::protocol::TokenUsageInfo;
use firefam_protocol::protocol::TurnEnvironmentSelection;
use firefam_protocol::protocol::W3cTraceContext;
use firefam_protocol::user_input::UserInput;
use firefam_thread_store::StoredThread;
use firefam_thread_store::StoredThreadHistory;
use firefam_thread_store::ThreadMetadataPatch;
use firefam_thread_store::ThreadStoreError;
use firefam_thread_store::ThreadStoreResult;
use firefam_utils_absolute_path::AbsolutePathBuf;
use rmcp::model::ReadResourceRequestParams;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::watch;

use firefam_rollout::state_db::StateDbHandle;

#[derive(Clone, Debug)]
pub struct ThreadConfigSnapshot {
    pub model: String,
    pub model_provider_id: String,
    pub service_tier: Option<String>,
    pub approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
    pub permission_profile: PermissionProfile,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub cwd: AbsolutePathBuf,
    pub workspace_roots: Vec<AbsolutePathBuf>,
    pub profile_workspace_roots: Vec<AbsolutePathBuf>,
    pub ephemeral: bool,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_summary: Option<ReasoningSummary>,
    pub personality: Option<Personality>,
    pub collaboration_mode: CollaborationMode,
    pub session_source: SessionSource,
    pub thread_source: Option<ThreadSource>,
}

impl ThreadConfigSnapshot {
    pub fn sandbox_policy(&self) -> SandboxPolicy {
        let file_system_sandbox_policy = self.permission_profile.file_system_sandbox_policy();
        firefam_sandboxing::compatibility_sandbox_policy_for_permission_profile(
            &self.permission_profile,
            &file_system_sandbox_policy,
            self.permission_profile.network_sandbox_policy(),
            self.cwd.as_path(),
        )
    }
}

/// Thread settings overrides that app-server validates before starting a turn.
#[derive(Clone, Default)]
pub struct FirefamThreadSettingsOverrides {
    pub cwd: Option<PathBuf>,
    pub workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub profile_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub sandbox_policy: Option<SandboxPolicy>,
    pub permission_profile: Option<PermissionProfile>,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub windows_sandbox_level: Option<WindowsSandboxLevel>,
    pub model: Option<String>,
    pub effort: Option<Option<ReasoningEffort>>,
    pub summary: Option<ReasoningSummary>,
    pub service_tier: Option<Option<String>>,
    pub collaboration_mode: Option<CollaborationMode>,
    pub personality: Option<Personality>,
}

pub struct FirefamThread {
    pub(crate) firefam: Firefam,
    pub(crate) session_source: SessionSource,
    session_configured: SessionConfiguredEvent,
    rollout_path: Option<PathBuf>,
    out_of_band_elicitation_count: Mutex<u64>,
}

/// Conduit for the bidirectional stream of messages that compose a thread
/// (formerly called a conversation) in Firefam.
impl FirefamThread {
    pub(crate) fn new(
        firefam: Firefam,
        session_configured: SessionConfiguredEvent,
        rollout_path: Option<PathBuf>,
        session_source: SessionSource,
    ) -> Self {
        Self {
            firefam,
            session_source,
            session_configured,
            rollout_path,
            out_of_band_elicitation_count: Mutex::new(0),
        }
    }

    pub async fn submit(&self, op: Op) -> FirefamResult<String> {
        self.firefam.submit(op).await
    }

    /// Returns the session telemetry handle for thread-scoped production instrumentation.
    pub fn session_telemetry(&self) -> SessionTelemetry {
        self.firefam.session.services.session_telemetry.clone()
    }

    pub async fn shutdown_and_wait(&self) -> FirefamResult<()> {
        self.firefam.shutdown_and_wait().await
    }

    /// Wait until the underlying session loop has terminated.
    pub async fn wait_until_terminated(&self) {
        self.firefam.session_loop_termination.clone().await;
    }

    pub(crate) async fn emit_thread_resume_lifecycle(&self) {
        for contributor in self
            .firefam
            .session
            .services
            .extensions
            .thread_lifecycle_contributors()
        {
            contributor
                .on_thread_resume(firefam_extension_api::ThreadResumeInput {
                    session_store: &self.firefam.session.services.session_extension_data,
                    thread_store: &self.firefam.session.services.thread_extension_data,
                })
                .await;
        }
    }

    pub async fn apply_goal_resume_runtime_effects(&self) -> anyhow::Result<()> {
        self.firefam
            .session
            .goal_runtime_apply(GoalRuntimeEvent::ThreadResumed)
            .await
    }

    pub async fn continue_active_goal_if_idle(&self) -> anyhow::Result<()> {
        self.firefam
            .session
            .goal_runtime_apply(GoalRuntimeEvent::MaybeContinueIfIdle)
            .await
    }

    pub async fn prepare_external_goal_mutation(&self) {
        if let Err(err) = self
            .firefam
            .session
            .goal_runtime_apply(GoalRuntimeEvent::ExternalMutationStarting)
            .await
        {
            tracing::warn!("failed to prepare external goal mutation: {err}");
        }
    }

    pub async fn apply_external_goal_set(&self, external_set: ExternalGoalSet) {
        if let Err(err) = self
            .firefam
            .session
            .goal_runtime_apply(GoalRuntimeEvent::ExternalSet { external_set })
            .await
        {
            tracing::warn!("failed to apply external goal status runtime effects: {err}");
        }
    }

    pub async fn apply_external_goal_clear(&self) {
        if let Err(err) = self
            .firefam
            .session
            .goal_runtime_apply(GoalRuntimeEvent::ExternalClear)
            .await
        {
            tracing::warn!("failed to apply external goal clear runtime effects: {err}");
        }
    }

    #[doc(hidden)]
    pub async fn ensure_rollout_materialized(&self) {
        self.firefam.session.ensure_rollout_materialized().await;
    }

    #[doc(hidden)]
    pub async fn flush_rollout(&self) -> std::io::Result<()> {
        self.firefam.session.flush_rollout().await
    }

    pub async fn submit_with_trace(
        &self,
        op: Op,
        trace: Option<W3cTraceContext>,
    ) -> FirefamResult<String> {
        self.firefam.submit_with_trace(op, trace).await
    }

    /// Persist whether this thread is eligible for future memory generation.
    pub async fn set_thread_memory_mode(&self, mode: ThreadMemoryMode) -> anyhow::Result<()> {
        self.firefam.set_thread_memory_mode(mode).await
    }

    pub async fn steer_input(
        &self,
        input: Vec<UserInput>,
        expected_turn_id: Option<&str>,
        responsesapi_client_metadata: Option<HashMap<String, String>>,
    ) -> Result<String, SteerInputError> {
        self.firefam
            .steer_input(input, expected_turn_id, responsesapi_client_metadata)
            .await
    }

    pub async fn set_app_server_client_info(
        &self,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        mcp_elicitations_auto_deny: bool,
    ) -> ConstraintResult<()> {
        self.firefam
            .set_app_server_client_info(
                app_server_client_name,
                app_server_client_version,
                mcp_elicitations_auto_deny,
            )
            .await
    }

    /// Preview persistent thread settings overrides without committing them.
    pub async fn preview_thread_settings_overrides(
        &self,
        overrides: FirefamThreadSettingsOverrides,
    ) -> ConstraintResult<ThreadConfigSnapshot> {
        let updates = self.thread_settings_update(overrides).await;
        self.firefam.session.preview_settings(&updates).await
    }

    async fn thread_settings_update(
        &self,
        overrides: FirefamThreadSettingsOverrides,
    ) -> SessionSettingsUpdate {
        let FirefamThreadSettingsOverrides {
            cwd,
            workspace_roots,
            profile_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox_policy,
            permission_profile,
            active_permission_profile,
            windows_sandbox_level,
            model,
            effort,
            summary,
            service_tier,
            collaboration_mode,
            personality,
        } = overrides;
        let collaboration_mode = if let Some(collaboration_mode) = collaboration_mode {
            collaboration_mode
        } else {
            self.firefam
                .session
                .collaboration_mode()
                .await
                .with_updates(model, effort, /*developer_instructions*/ None)
        };

        SessionSettingsUpdate {
            cwd,
            workspace_roots,
            profile_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox_policy,
            permission_profile,
            active_permission_profile,
            windows_sandbox_level,
            collaboration_mode: Some(collaboration_mode),
            reasoning_summary: summary,
            service_tier,
            personality,
            ..Default::default()
        }
    }

    /// Use sparingly: this is intended to be removed soon.
    pub async fn submit_with_id(&self, sub: Submission) -> FirefamResult<()> {
        self.firefam.submit_with_id(sub).await
    }

    pub async fn next_event(&self) -> FirefamResult<Event> {
        self.firefam.next_event().await
    }

    pub async fn agent_status(&self) -> AgentStatus {
        self.firefam.agent_status().await
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<AgentStatus> {
        self.firefam.agent_status.clone()
    }

    /// Returns the complete token usage snapshot currently cached for this thread.
    ///
    /// This accessor is intentionally narrower than direct session access: it lets
    /// app-server lifecycle paths replay restored usage after resume or fork without
    /// exposing broader session mutation authority. A caller that only reads
    /// `total_token_usage` would drop last-turn usage and make the v2
    /// `thread/tokenUsage/updated` payload incomplete.
    pub async fn token_usage_info(&self) -> Option<TokenUsageInfo> {
        self.firefam.session.token_usage_info().await
    }

    /// Records a user-role session-prefix message without creating a new user turn boundary.
    pub(crate) async fn inject_user_message_without_turn(&self, message: String) {
        let message = ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText { text: message }],
            phase: None,
        };
        let pending_item = match pending_message_input_item(&message) {
            Ok(pending_item) => pending_item,
            Err(err) => {
                debug_assert!(false, "session-prefix message append should succeed: {err}");
                return;
            }
        };
        if self
            .firefam
            .session
            .inject_response_items(vec![pending_item])
            .await
            .is_err()
        {
            let turn_context = self.firefam.session.new_default_turn().await;
            self.firefam
                .session
                .record_conversation_items(turn_context.as_ref(), &[message])
                .await;
        }
    }

    /// Append a prebuilt message to the thread history without treating it as a user turn.
    ///
    /// If the thread already has an active turn, the message is queued as pending input for that
    /// turn. Otherwise it is queued at session scope and a regular turn is started so the agent
    /// can consume that pending input through the normal turn pipeline.
    #[cfg(test)]
    pub(crate) async fn append_message(&self, message: ResponseItem) -> FirefamResult<String> {
        let submission_id = uuid::Uuid::new_v4().to_string();
        let pending_item = pending_message_input_item(&message)?;
        if let Err(items) = self
            .firefam
            .session
            .inject_response_items(vec![pending_item])
            .await
        {
            self.firefam
                .session
                .input_queue
                .queue_response_items_for_next_turn(items)
                .await;
            self.firefam
                .session
                .maybe_start_turn_for_pending_work()
                .await;
        }

        Ok(submission_id)
    }

    /// Append raw Responses API items to the thread's model-visible history.
    pub async fn inject_response_items(&self, items: Vec<ResponseItem>) -> FirefamResult<()> {
        if items.is_empty() {
            return Err(FirefamErr::InvalidRequest(
                "items must not be empty".to_string(),
            ));
        }

        let turn_context = self.firefam.session.new_default_turn().await;
        if self
            .firefam
            .session
            .reference_context_item()
            .await
            .is_none()
        {
            self.firefam
                .session
                .record_context_updates_and_set_reference_context_item(turn_context.as_ref())
                .await;
        }
        self.firefam
            .session
            .record_conversation_items(turn_context.as_ref(), &items)
            .await;
        self.firefam.session.flush_rollout().await?;
        Ok(())
    }

    pub fn rollout_path(&self) -> Option<PathBuf> {
        self.rollout_path.clone()
    }

    pub fn session_configured(&self) -> SessionConfiguredEvent {
        self.session_configured.clone()
    }

    pub(crate) fn is_running(&self) -> bool {
        !self.firefam.tx_sub.is_closed()
    }

    pub async fn guardian_trunk_rollout_path(&self) -> Option<PathBuf> {
        self.firefam
            .session
            .guardian_review_session
            .trunk_rollout_path()
            .await
    }

    pub async fn load_history(
        &self,
        include_archived: bool,
    ) -> ThreadStoreResult<StoredThreadHistory> {
        let live_thread = self
            .firefam
            .session
            .live_thread_for_persistence("load history")
            .map_err(|err| ThreadStoreError::Internal {
                message: err.to_string(),
            })?;
        live_thread.load_history(include_archived).await
    }

    pub async fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreResult<StoredThread> {
        let live_thread = self
            .firefam
            .session
            .live_thread_for_persistence("read thread")
            .map_err(|err| ThreadStoreError::Internal {
                message: err.to_string(),
            })?;
        live_thread
            .read_thread(include_archived, include_history)
            .await
    }

    pub async fn update_thread_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreResult<StoredThread> {
        let live_thread = self
            .firefam
            .session
            .live_thread_for_persistence("update thread metadata")
            .map_err(|err| ThreadStoreError::Internal {
                message: err.to_string(),
            })?;
        live_thread.update_metadata(patch, include_archived).await
    }

    pub fn state_db(&self) -> Option<StateDbHandle> {
        self.firefam.state_db()
    }

    pub async fn config_snapshot(&self) -> ThreadConfigSnapshot {
        self.firefam.thread_config_snapshot().await
    }

    pub async fn config(&self) -> Arc<crate::config::Config> {
        self.firefam.session.get_config().await
    }

    /// Refresh the thread's layer-backed user config state from a caller-supplied
    /// config snapshot. Thread-scoped layers and session-static settings remain
    /// unchanged.
    pub async fn refresh_runtime_config(&self, next_config: crate::config::Config) {
        self.firefam
            .session
            .refresh_runtime_config(next_config)
            .await;
    }

    pub async fn environment_selections(&self) -> Vec<TurnEnvironmentSelection> {
        self.firefam.thread_environment_selections().await
    }

    pub async fn read_mcp_resource(
        &self,
        server: &str,
        uri: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let result = self
            .firefam
            .session
            .read_resource(
                server,
                ReadResourceRequestParams {
                    meta: None,
                    uri: uri.to_string(),
                },
            )
            .await?;

        Ok(serde_json::to_value(result)?)
    }

    pub async fn call_mcp_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: Option<serde_json::Value>,
        meta: Option<serde_json::Value>,
    ) -> anyhow::Result<CallToolResult> {
        self.firefam
            .session
            .call_tool(server, tool, arguments, meta)
            .await
    }

    pub fn enabled(&self, feature: Feature) -> bool {
        self.firefam.enabled(feature)
    }

    pub async fn increment_out_of_band_elicitation_count(&self) -> FirefamResult<u64> {
        let mut guard = self.out_of_band_elicitation_count.lock().await;
        let was_zero = *guard == 0;
        *guard = guard.checked_add(1).ok_or_else(|| {
            FirefamErr::Fatal("out-of-band elicitation count overflowed".to_string())
        })?;

        if was_zero {
            self.firefam
                .session
                .set_out_of_band_elicitation_pause_state(/*paused*/ true);
        }

        Ok(*guard)
    }

    pub async fn decrement_out_of_band_elicitation_count(&self) -> FirefamResult<u64> {
        let mut guard = self.out_of_band_elicitation_count.lock().await;
        if *guard == 0 {
            return Err(FirefamErr::InvalidRequest(
                "out-of-band elicitation count is already zero".to_string(),
            ));
        }

        *guard -= 1;
        let now_zero = *guard == 0;
        if now_zero {
            self.firefam
                .session
                .set_out_of_band_elicitation_pause_state(/*paused*/ false);
        }

        Ok(*guard)
    }
}

fn pending_message_input_item(message: &ResponseItem) -> FirefamResult<ResponseInputItem> {
    match message {
        ResponseItem::Message {
            role,
            content,
            phase,
            ..
        } => Ok(ResponseInputItem::Message {
            role: role.clone(),
            content: content.clone(),
            phase: phase.clone(),
        }),
        _ => Err(FirefamErr::InvalidRequest(
            "append_message only supports ResponseItem::Message".to_string(),
        )),
    }
}
