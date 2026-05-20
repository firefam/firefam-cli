//! Implements the MultiAgentV2 collaboration tool surface.

use crate::agent::AgentStatus;
use crate::agent::agent_resolver::resolve_agent_target;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::multi_agents_common::*;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use firefam_protocol::AgentPath;
use firefam_protocol::firefamai_models::ReasoningEffort;
use firefam_protocol::models::ResponseInputItem;
use firefam_protocol::protocol::CollabAgentInteractionBeginEvent;
use firefam_protocol::protocol::CollabAgentInteractionEndEvent;
use firefam_protocol::protocol::CollabAgentSpawnBeginEvent;
use firefam_protocol::protocol::CollabAgentSpawnEndEvent;
use firefam_protocol::protocol::CollabCloseBeginEvent;
use firefam_protocol::protocol::CollabCloseEndEvent;
use firefam_protocol::protocol::CollabWaitingBeginEvent;
use firefam_protocol::protocol::CollabWaitingEndEvent;
use firefam_protocol::user_input::UserInput;
use firefam_tools::ToolName;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;

pub(crate) use close_agent::Handler as CloseAgentHandler;
pub(crate) use followup_task::Handler as FollowupTaskHandler;
pub(crate) use list_agents::Handler as ListAgentsHandler;
pub(crate) use send_message::Handler as SendMessageHandler;
pub(crate) use spawn::Handler as SpawnAgentHandler;
pub(crate) use wait::Handler as WaitAgentHandler;

mod close_agent;
mod followup_task;
mod list_agents;
mod message_tool;
mod send_message;
mod spawn;
pub(crate) mod wait;
