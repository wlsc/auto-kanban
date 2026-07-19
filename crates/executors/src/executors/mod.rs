use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use command_group::AsyncGroupChild;
use enum_dispatch::enum_dispatch;
use futures::stream::BoxStream;
use futures_io::Error as FuturesIoError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use strum_macros::{AsRefStr, Display, EnumDiscriminants, EnumString, VariantNames};
use thiserror::Error;
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

#[cfg(feature = "qa-mode")]
use crate::executors::qa_mock::QaMockExecutor;
use crate::{
    actions::{ExecutorAction, review::RepoReviewContext},
    approvals::ExecutorApprovalService,
    command::CommandBuildError,
    env::ExecutionEnv,
    executors::{
        amp::Amp, claude::ClaudeCode, codex::Codex, copilot::Copilot, cursor::CursorAgent,
        droid::Droid, gemini::Gemini, opencode::Opencode, qwen::QwenCode,
    },
    logs::utils::patch,
    mcp_config::McpConfig,
};

pub mod acp;
pub mod amp;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod droid;
pub mod gemini;
pub mod opencode;
#[cfg(feature = "qa-mode")]
pub mod qa_mock;
pub mod qwen;
pub mod utils;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct SlashCommandDescription {
    /// Command name without the leading slash, e.g. `help` for `/help`.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(use_ts_enum)]
pub enum BaseAgentCapability {
    SessionFork,
    /// Agent requires a setup script before it can run (e.g., login, installation)
    SetupHelper,
    /// Agent reports context/token usage information
    ContextUsage,
}

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Follow-up is not supported: {0}")]
    FollowUpNotSupported(String),
    #[error(transparent)]
    SpawnError(#[from] FuturesIoError),
    #[error("Unknown executor type: {0}")]
    UnknownExecutorType(String),
    #[error("I/O error: {0}")]
    Io(std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    ExecutorApprovalError(#[from] crate::approvals::ExecutorApprovalError),
    #[error(transparent)]
    CommandBuild(#[from] CommandBuildError),
    #[error("Executable `{program}` not found in PATH")]
    ExecutableNotFound { program: String },
    #[error("Setup helper not supported")]
    SetupHelperNotSupported,
    #[error("Auth required: {0}")]
    AuthRequired(String),
}

/// Cross-executor reasoning effort level chosen by the user when starting a task.
///
/// Executors expose different native effort scales; this shared enum is mapped
/// onto each executor's own configuration by [`CodingAgent::apply_reasoning_effort`].
/// Values mirror the Claude Code `--effort` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[ts(use_ts_enum)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[enum_dispatch]
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, TS, Display, EnumDiscriminants, VariantNames,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[strum_discriminants(
    name(BaseCodingAgent),
    // Only add Hash; Eq/PartialEq are already provided by EnumDiscriminants.
    derive(EnumString, Hash, strum_macros::Display, Serialize, Deserialize, TS, Type),
    strum(serialize_all = "SCREAMING_SNAKE_CASE"),
    ts(use_ts_enum),
    serde(rename_all = "SCREAMING_SNAKE_CASE"),
    sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")
)]
pub enum CodingAgent {
    ClaudeCode,
    Amp,
    Gemini,
    Codex,
    Opencode,
    #[serde(alias = "CURSOR")]
    #[strum_discriminants(serde(alias = "CURSOR"))]
    #[strum_discriminants(strum(serialize = "CURSOR", serialize = "CURSOR_AGENT"))]
    CursorAgent,
    QwenCode,
    Copilot,
    Droid,
    #[cfg(feature = "qa-mode")]
    QaMock(QaMockExecutor),
}

impl CodingAgent {
    pub fn get_mcp_config(&self) -> McpConfig {
        match self {
            Self::Codex(_) => McpConfig::new(
                vec!["mcp_servers".to_string()],
                serde_json::json!({
                    "mcp_servers": {}
                }),
                self.preconfigured_mcp(),
                true,
            ),
            Self::Amp(_) => McpConfig::new(
                vec!["amp.mcpServers".to_string()],
                serde_json::json!({
                    "amp.mcpServers": {}
                }),
                self.preconfigured_mcp(),
                false,
            ),
            Self::Opencode(_) => McpConfig::new(
                vec!["mcp".to_string()],
                serde_json::json!({
                    "mcp": {},
                    "$schema": "https://opencode.ai/config.json"
                }),
                self.preconfigured_mcp(),
                false,
            ),
            Self::Droid(_) => McpConfig::new(
                vec!["mcpServers".to_string()],
                serde_json::json!({
                    "mcpServers": {}
                }),
                self.preconfigured_mcp(),
                false,
            ),
            _ => McpConfig::new(
                vec!["mcpServers".to_string()],
                serde_json::json!({
                    "mcpServers": {}
                }),
                self.preconfigured_mcp(),
                false,
            ),
        }
    }

    pub fn supports_mcp(&self) -> bool {
        self.default_mcp_config_path().is_some()
    }

    pub fn capabilities(&self) -> Vec<BaseAgentCapability> {
        match self {
            Self::ClaudeCode(_) => vec![
                BaseAgentCapability::SessionFork,
                BaseAgentCapability::ContextUsage,
            ],
            Self::Opencode(_) => vec![
                BaseAgentCapability::SessionFork,
                BaseAgentCapability::ContextUsage,
            ],
            Self::Codex(_) => vec![
                BaseAgentCapability::SessionFork,
                BaseAgentCapability::SetupHelper,
                BaseAgentCapability::ContextUsage,
            ],
            Self::Amp(_) | Self::Gemini(_) | Self::QwenCode(_) => {
                vec![BaseAgentCapability::SessionFork]
            }
            Self::CursorAgent(_) => vec![BaseAgentCapability::SetupHelper],
            Self::Copilot(_) | Self::Droid(_) => vec![],
            #[cfg(feature = "qa-mode")]
            Self::QaMock(_) => vec![], // QA mock doesn't need special capabilities
        }
    }

    /// Apply a user-selected reasoning effort onto this agent's native config.
    ///
    /// Each executor exposes a different effort scale, so the shared
    /// [`EffortLevel`] is clamped onto the closest supported value. Executors
    /// without a reasoning-effort concept (Amp, Gemini, Cursor, …) ignore it.
    pub fn apply_reasoning_effort(&mut self, effort: EffortLevel) {
        use crate::executors::{codex::ReasoningEffort, droid::ReasoningEffortLevel};

        match self {
            Self::ClaudeCode(agent) => agent.reasoning_effort = Some(effort),
            Self::Codex(agent) => {
                agent.model_reasoning_effort = Some(match effort {
                    EffortLevel::Low => ReasoningEffort::Low,
                    EffortLevel::Medium => ReasoningEffort::Medium,
                    EffortLevel::High => ReasoningEffort::High,
                    // Codex tops out at xhigh.
                    EffortLevel::Xhigh | EffortLevel::Max => ReasoningEffort::Xhigh,
                });
            }
            Self::Droid(agent) => {
                agent.reasoning_effort = Some(match effort {
                    EffortLevel::Low => ReasoningEffortLevel::Low,
                    EffortLevel::Medium => ReasoningEffortLevel::Medium,
                    // Droid tops out at high.
                    EffortLevel::High | EffortLevel::Xhigh | EffortLevel::Max => {
                        ReasoningEffortLevel::High
                    }
                });
            }
            _ => {}
        }
    }

    /// The reasoning effort currently configured on this agent, as a display
    /// label, if the executor supports one and a value is set.
    pub fn reasoning_effort_label(&self) -> Option<String> {
        match self {
            Self::ClaudeCode(agent) => agent.reasoning_effort.map(|e| e.as_ref().to_string()),
            Self::Codex(agent) => agent
                .model_reasoning_effort
                .as_ref()
                .map(|e| e.as_ref().to_string()),
            Self::Droid(agent) => agent
                .reasoning_effort
                .as_ref()
                .map(|e| e.as_ref().to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum AvailabilityInfo {
    LoginDetected { last_auth_timestamp: i64 },
    InstallationFound,
    NotFound,
}

impl AvailabilityInfo {
    pub fn is_available(&self) -> bool {
        matches!(
            self,
            AvailabilityInfo::LoginDetected { .. } | AvailabilityInfo::InstallationFound
        )
    }
}

#[async_trait]
#[enum_dispatch(CodingAgent)]
pub trait StandardCodingAgentExecutor {
    fn use_approvals(&mut self, _approvals: Arc<dyn ExecutorApprovalService>) {}

    async fn available_slash_commands(
        &self,
        _workdir: &Path,
    ) -> Result<BoxStream<'static, json_patch::Patch>, ExecutorError> {
        Ok(Box::pin(futures::stream::once(async move {
            patch::slash_commands(Vec::new(), false, None)
        })))
    }

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError>;

    /// Continue a session, optionally resetting to a specific message.
    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError>;

    async fn spawn_review(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        match session_id {
            Some(id) => {
                self.spawn_follow_up(current_dir, prompt, id, None, env)
                    .await
            }
            None => self.spawn(current_dir, prompt, env).await,
        }
    }

    fn normalize_logs(&self, _raw_logs_event_store: Arc<MsgStore>, _worktree_path: &Path);

    // MCP configuration methods
    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf>;

    async fn get_setup_helper_action(&self) -> Result<ExecutorAction, ExecutorError> {
        Err(ExecutorError::SetupHelperNotSupported)
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        let config_files_found = self
            .default_mcp_config_path()
            .map(|path| path.exists())
            .unwrap_or(false);

        if config_files_found {
            AvailabilityInfo::InstallationFound
        } else {
            AvailabilityInfo::NotFound
        }
    }
}

/// Result communicated through the exit signal
#[derive(Debug, Clone, Copy)]
pub enum ExecutorExitResult {
    /// Process completed successfully (exit code 0)
    Success,
    /// Process should be marked as failed (non-zero exit)
    Failure,
}

/// Optional exit notification from an executor.
/// When this receiver resolves, the container should gracefully stop the process
/// and mark it according to the result.
pub type ExecutorExitSignal = tokio::sync::oneshot::Receiver<ExecutorExitResult>;

/// Cancellation token for requesting graceful shutdown of an executor.
/// When cancelled, the executor should attempt to cancel gracefully before being killed.
pub type CancellationToken = tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct SpawnedChild {
    pub child: AsyncGroupChild,
    /// Executor → Container: signals when executor wants to exit
    pub exit_signal: Option<ExecutorExitSignal>,
    /// Container → Executor: signals when container wants to cancel the execution
    pub cancel: Option<CancellationToken>,
}

impl From<AsyncGroupChild> for SpawnedChild {
    fn from(child: AsyncGroupChild) -> Self {
        Self {
            child,
            exit_signal: None,
            cancel: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema)]
#[serde(transparent)]
#[schemars(
    title = "Append Prompt",
    description = "Extra text appended to the prompt",
    extend("format" = "textarea")
)]
#[derive(Default)]
pub struct AppendPrompt(pub Option<String>);

impl AppendPrompt {
    pub fn get(&self) -> Option<String> {
        self.0.clone()
    }

    pub fn combine_prompt(&self, prompt: &str) -> String {
        match self {
            AppendPrompt(Some(value)) => format!("{prompt}{value}"),
            AppendPrompt(None) => prompt.to_string(),
        }
    }
}

pub fn build_review_prompt(
    context: Option<&[RepoReviewContext]>,
    additional_prompt: Option<&str>,
) -> String {
    let mut prompt = String::from("Please review the code changes.\n\n");

    if let Some(repos) = context {
        for repo in repos {
            prompt.push_str(&format!("Repository: {}\n", repo.repo_name));
            prompt.push_str(&format!(
                "Review all changes from base commit {} to HEAD.\n",
                repo.base_commit
            ));
            prompt.push_str(&format!(
                "Use `git diff {}..HEAD` to see the changes.\n",
                repo.base_commit
            ));
            prompt.push('\n');
        }
    }

    if let Some(additional) = additional_prompt {
        prompt.push_str(additional);
    }

    prompt
}

pub struct SolutionContext {
    pub label: String,
    pub executor_used: Option<String>,
    pub container_ref: String,
    pub repo_paths: Vec<(String, String)>,
    pub agent_summaries: Vec<String>,
}

pub fn build_comparison_task_description(
    task_description: Option<&str>,
    solutions: &[SolutionContext],
    additional_prompt: Option<&str>,
) -> String {
    let n = solutions.len();
    let mut desc = format!(
        "You are comparing {n} solutions to the same task. Each solution exists as a \
         worktree on the local filesystem. Navigate to each solution's path, read the \
         code, and evaluate which one best achieves the goal.\n\n"
    );

    if let Some(td) = task_description {
        desc.push_str("## Task Goal\n\n");
        desc.push_str(td);
        desc.push_str("\n\n");
    }

    for (i, sol) in solutions.iter().enumerate() {
        let letter = (b'A' + i as u8) as char;
        let executor_label = sol.executor_used.as_deref().unwrap_or("Unknown");
        desc.push_str(&format!(
            "---\n## Solution {letter} — {label} ({executor_label})\n\n",
            label = sol.label
        ));
        desc.push_str(&format!("**Worktree path:** `{}`\n", sol.container_ref));
        desc.push_str("**Repos:**\n");
        for (repo_name, target_branch) in &sol.repo_paths {
            desc.push_str(&format!(
                "- `{}/{}` (target branch: {})\n",
                sol.container_ref, repo_name, target_branch
            ));
        }
        desc.push('\n');

        if !sol.agent_summaries.is_empty() {
            desc.push_str("### Agent Reasoning\n\n");
            for summary in &sol.agent_summaries {
                desc.push_str(summary);
                desc.push_str("\n\n");
            }
        }
    }

    desc.push_str("---\n## Instructions\n\n");
    desc.push_str(
        "For each solution, navigate to its worktree path and examine the code changes. \
         Compare against the target branch to understand what was changed. Analyze:\n\
         1. **Correctness** — Does it correctly solve the task goal?\n\
         2. **Completeness** — Are all requirements addressed?\n\
         3. **Code Quality** — Is the code clean, maintainable, and well-structured?\n\
         4. **Approach / Design** — Is the overall approach sound?\n\n\
         Compare the solutions head-to-head and recommend the BEST one with clear \
         justification.\n\n",
    );

    if let Some(additional) = additional_prompt {
        desc.push_str("## Additional Instructions\n\n");
        desc.push_str(additional);
        desc.push('\n');
    }

    desc
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_cursor_agent_deserialization() {
        // Test that CURSOR_AGENT is accepted
        let result = BaseCodingAgent::from_str("CURSOR_AGENT");
        assert!(result.is_ok(), "CURSOR_AGENT should be valid");
        assert_eq!(result.unwrap(), BaseCodingAgent::CursorAgent);

        // Test that legacy CURSOR is still accepted for backwards compatibility
        let result = BaseCodingAgent::from_str("CURSOR");
        assert!(
            result.is_ok(),
            "CURSOR should be valid for backwards compatibility"
        );
        assert_eq!(result.unwrap(), BaseCodingAgent::CursorAgent);

        // Test serde deserialization for CURSOR_AGENT
        let result: Result<BaseCodingAgent, _> = serde_json::from_str(r#""CURSOR_AGENT""#);
        assert!(result.is_ok(), "CURSOR_AGENT should deserialize via serde");
        assert_eq!(result.unwrap(), BaseCodingAgent::CursorAgent);

        // Test serde deserialization for legacy CURSOR
        let result: Result<BaseCodingAgent, _> = serde_json::from_str(r#""CURSOR""#);
        assert!(result.is_ok(), "CURSOR should deserialize via serde");
        assert_eq!(result.unwrap(), BaseCodingAgent::CursorAgent);
    }

    #[test]
    fn test_apply_reasoning_effort_maps_and_clamps_per_executor() {
        use crate::executors::{
            claude::ClaudeCode, codex::Codex, codex::ReasoningEffort, droid::Droid,
            droid::ReasoningEffortLevel,
        };

        // Claude passes the level through unchanged.
        let mut claude = CodingAgent::ClaudeCode(serde_json::from_str::<ClaudeCode>("{}").unwrap());
        claude.apply_reasoning_effort(EffortLevel::Max);
        assert_eq!(claude.reasoning_effort_label().as_deref(), Some("max"));

        // Codex clamps `max` down to its top level, `xhigh`.
        let mut codex = CodingAgent::Codex(serde_json::from_str::<Codex>("{}").unwrap());
        codex.apply_reasoning_effort(EffortLevel::Max);
        match &codex {
            CodingAgent::Codex(c) => {
                assert_eq!(c.model_reasoning_effort, Some(ReasoningEffort::Xhigh))
            }
            _ => unreachable!(),
        }

        // Droid clamps anything above `high` to `high`.
        let mut droid =
            CodingAgent::Droid(serde_json::from_str::<Droid>(r#"{"autonomy":"high"}"#).unwrap());
        droid.apply_reasoning_effort(EffortLevel::Xhigh);
        match &droid {
            CodingAgent::Droid(d) => {
                assert_eq!(d.reasoning_effort, Some(ReasoningEffortLevel::High))
            }
            _ => unreachable!(),
        }
    }
}
