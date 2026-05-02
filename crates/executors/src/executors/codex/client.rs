use std::{
    borrow::Cow,
    collections::VecDeque,
    io,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use codex_app_server_protocol::{
    ApplyPatchApprovalResponse, ChatgptAuthTokensRefreshResponse, ClientInfo, ClientNotification,
    ClientRequest,
    CommandExecutionApprovalDecision, CommandExecutionRequestApprovalResponse,
    DynamicToolCallOutputContentItem, DynamicToolCallResponse, FileChangeApprovalDecision,
    ExecCommandApprovalResponse, FileChangeRequestApprovalResponse, GetAuthStatusParams,
    GetAuthStatusResponse,
    GrantedPermissionProfile, InitializeCapabilities, InitializeParams, InitializeResponse,
    JSONRPCError,
    JSONRPCNotification, JSONRPCRequest, JSONRPCResponse, ListMcpServerStatusParams,
    ListMcpServerStatusResponse, McpServerElicitationAction, McpServerElicitationRequestResponse,
    PermissionGrantScope, PermissionsRequestApprovalResponse, RequestId, RequestPermissionProfile,
    ReviewStartParams, ReviewStartResponse, ReviewTarget, ServerRequest, ThreadResumeParams,
    ThreadResumeResponse, ThreadStartParams, ThreadStartResponse, ToolRequestUserInputAnswer,
    ToolRequestUserInputResponse, TurnStartParams, TurnStartResponse, UserInput,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{self, Value};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;
use workspace_utils::approvals::ApprovalStatus;

use super::jsonrpc::{JsonRpcCallbacks, JsonRpcPeer};
use crate::{
    approvals::{ExecutorApprovalError, ExecutorApprovalService},
    env::RepoContext,
    executors::{ExecutorError, codex::normalize_logs::Approval},
};

pub struct AppServerClient {
    rpc: OnceLock<JsonRpcPeer>,
    log_writer: LogWriter,
    approvals: Option<Arc<dyn ExecutorApprovalService>>,
    thread_id: Mutex<Option<String>>,
    pending_feedback: Mutex<VecDeque<String>>,
    auto_approve: bool,
    repo_context: RepoContext,
    commit_reminder: bool,
    commit_reminder_prompt: String,
    commit_reminder_sent: AtomicBool,
    cancel: CancellationToken,
}

impl AppServerClient {
    pub fn new(
        log_writer: LogWriter,
        approvals: Option<Arc<dyn ExecutorApprovalService>>,
        auto_approve: bool,
        repo_context: RepoContext,
        commit_reminder: bool,
        commit_reminder_prompt: String,
        cancel: CancellationToken,
    ) -> Arc<Self> {
        Arc::new(Self {
            rpc: OnceLock::new(),
            log_writer,
            approvals,
            auto_approve,
            thread_id: Mutex::new(None),
            pending_feedback: Mutex::new(VecDeque::new()),
            repo_context,
            commit_reminder,
            commit_reminder_prompt,
            commit_reminder_sent: AtomicBool::new(false),
            cancel,
        })
    }

    pub fn connect(&self, peer: JsonRpcPeer) {
        let _ = self.rpc.set(peer);
    }

    fn rpc(&self) -> &JsonRpcPeer {
        self.rpc.get().expect("Codex RPC peer not attached")
    }

    pub fn log_writer(&self) -> &LogWriter {
        &self.log_writer
    }

    pub async fn initialize(&self) -> Result<(), ExecutorError> {
        let request = ClientRequest::Initialize {
            request_id: self.next_request_id(),
            params: InitializeParams {
                client_info: ClientInfo {
                    name: "vibe-codex-executor".to_string(),
                    title: None,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                capabilities: Some(InitializeCapabilities {
                    experimental_api: true,
                    opt_out_notification_methods: None,
                }),
            },
        };

        self.send_request::<InitializeResponse>(request, "initialize")
            .await?;
        self.send_message(&ClientNotification::Initialized).await
    }

    pub async fn start_thread(
        &self,
        params: ThreadStartParams,
    ) -> Result<ThreadStartResponse, ExecutorError> {
        let request = ClientRequest::ThreadStart {
            request_id: self.next_request_id(),
            params,
        };
        self.send_request(request, "thread/start").await
    }

    pub async fn resume_thread(
        &self,
        rollout_path: std::path::PathBuf,
        overrides: ThreadStartParams,
    ) -> Result<ThreadResumeResponse, ExecutorError> {
        let request = ClientRequest::ThreadResume {
            request_id: self.next_request_id(),
            params: ThreadResumeParams {
                thread_id: String::new(),
                path: Some(rollout_path),
                model: overrides.model,
                model_provider: overrides.model_provider,
                service_tier: overrides.service_tier,
                cwd: overrides.cwd,
                approval_policy: overrides.approval_policy,
                approvals_reviewer: overrides.approvals_reviewer,
                sandbox: overrides.sandbox,
                permission_profile: overrides.permission_profile,
                config: overrides.config,
                base_instructions: overrides.base_instructions,
                developer_instructions: overrides.developer_instructions,
                personality: overrides.personality,
                history: None,
                exclude_turns: false,
                persist_extended_history: overrides.persist_extended_history,
            },
        };
        self.send_request(request, "thread/resume").await
    }

    pub async fn start_turn(
        &self,
        thread_id: String,
        message: String,
    ) -> Result<TurnStartResponse, ExecutorError> {
        let request = ClientRequest::TurnStart {
            request_id: self.next_request_id(),
            params: TurnStartParams {
                thread_id,
                input: vec![UserInput::Text {
                    text: message,
                    text_elements: vec![],
                }],
                responsesapi_client_metadata: None,
                environments: None,
                cwd: None,
                approval_policy: None,
                approvals_reviewer: None,
                sandbox_policy: None,
                permission_profile: None,
                model: None,
                service_tier: None,
                effort: None,
                summary: None,
                personality: None,
                output_schema: None,
                collaboration_mode: None,
            },
        };
        self.send_request(request, "turn/start").await
    }

    pub async fn get_auth_status(&self) -> Result<GetAuthStatusResponse, ExecutorError> {
        let request = ClientRequest::GetAuthStatus {
            request_id: self.next_request_id(),
            params: GetAuthStatusParams {
                include_token: Some(true),
                refresh_token: Some(false),
            },
        };
        self.send_request(request, "getAuthStatus").await
    }

    pub async fn start_review(
        &self,
        thread_id: String,
        target: ReviewTarget,
    ) -> Result<ReviewStartResponse, ExecutorError> {
        let request = ClientRequest::ReviewStart {
            request_id: self.next_request_id(),
            params: ReviewStartParams {
                thread_id,
                target,
                delivery: None,
            },
        };
        self.send_request(request, "reviewStart").await
    }

    pub async fn list_mcp_server_status(
        &self,
        cursor: Option<String>,
    ) -> Result<ListMcpServerStatusResponse, ExecutorError> {
        let request = ClientRequest::McpServerStatusList {
            request_id: self.next_request_id(),
            params: ListMcpServerStatusParams {
                cursor,
                limit: None,
                detail: None,
            },
        };
        self.send_request(request, "mcpServerStatus/list").await
    }

    async fn handle_server_request(
        &self,
        peer: &JsonRpcPeer,
        request: ServerRequest,
    ) -> Result<(), ExecutorError> {
        match request {
            ServerRequest::ApplyPatchApproval { request_id, params } => {
                let input = serde_json::to_value(&params)
                    .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?;
                let status = self
                    .request_tool_approval("edit", input, &params.call_id)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            "Codex apply_patch approval failed for call_id={}: {err}",
                            params.call_id
                        );
                        err
                    })?;
                let (decision, feedback) = self.review_decision(&status);
                let response = ApplyPatchApprovalResponse { decision };
                send_server_response(peer, request_id, response).await?;
                if let Some(message) = feedback {
                    tracing::debug!("queueing patch denial feedback: {message}");
                    self.enqueue_feedback(message).await;
                }
                Ok(())
            }
            ServerRequest::ExecCommandApproval { request_id, params } => {
                let input = serde_json::to_value(&params)
                    .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?;
                let status = self
                    .request_tool_approval("bash", input, &params.call_id)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            "Codex exec command approval failed for call_id={}: {err}",
                            params.call_id
                        );
                        err
                    })?;
                let (decision, feedback) = self.review_decision(&status);
                let response = ExecCommandApprovalResponse { decision };
                send_server_response(peer, request_id, response).await?;
                if let Some(message) = feedback {
                    tracing::debug!("queueing exec denial feedback: {message}");
                    self.enqueue_feedback(message).await;
                }
                Ok(())
            }
            ServerRequest::CommandExecutionRequestApproval { request_id, params } => {
                let input = serde_json::to_value(&params)
                    .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?;
                let status = self
                    .request_tool_approval("bash", input, &params.item_id)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            "Codex command approval failed for item_id={}: {err}",
                            params.item_id
                        );
                        err
                    })?;
                self.log_writer
                    .log_raw(
                        &Approval::approval_response(
                            params.item_id.clone(),
                            "codex.command_execution".to_string(),
                            status.clone(),
                        )
                        .raw(),
                    )
                    .await?;
                let (decision, feedback) = self.command_approval_decision(&status);
                let response = CommandExecutionRequestApprovalResponse { decision };
                send_server_response(peer, request_id, response).await?;
                if let Some(message) = feedback {
                    tracing::debug!("queueing command denial feedback: {message}");
                    self.enqueue_feedback(message).await;
                }
                Ok(())
            }
            ServerRequest::FileChangeRequestApproval { request_id, params } => {
                let input = serde_json::to_value(&params)
                    .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?;
                let status = self
                    .request_tool_approval("edit", input, &params.item_id)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            "Codex file-change approval failed for item_id={}: {err}",
                            params.item_id
                        );
                        err
                    })?;
                self.log_writer
                    .log_raw(
                        &Approval::approval_response(
                            params.item_id.clone(),
                            "codex.file_change".to_string(),
                            status.clone(),
                        )
                        .raw(),
                    )
                    .await?;

                let (decision, feedback) = self.file_change_approval_decision(&status);
                let response = FileChangeRequestApprovalResponse { decision };
                send_server_response(peer, request_id, response).await?;
                if let Some(message) = feedback {
                    tracing::debug!("queueing file-change denial feedback: {message}");
                    self.enqueue_feedback(message).await;
                }
                Ok(())
            }
            ServerRequest::ToolRequestUserInput { request_id, params } => {
                let answers = params
                    .questions
                    .iter()
                    .map(|question| {
                        let selected = question
                            .options
                            .as_ref()
                            .and_then(|options| options.first())
                            .map(|option| option.label.clone())
                            .map(|label| vec![label])
                            .unwrap_or_default();
                        (
                            question.id.clone(),
                            ToolRequestUserInputAnswer { answers: selected },
                        )
                    })
                    .collect();
                let response = ToolRequestUserInputResponse { answers };
                send_server_response(peer, request_id, response).await
            }
            ServerRequest::McpServerElicitationRequest {
                request_id,
                params: _,
            } => {
                let response = McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Cancel,
                    content: None,
                    meta: None,
                };
                send_server_response(peer, request_id, response).await
            }
            ServerRequest::PermissionsRequestApproval { request_id, params } => {
                let input = serde_json::to_value(&params)
                    .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?;
                let status = self
                    .request_tool_approval("permissions", input, &params.item_id)
                    .await
                    .map_err(|err| {
                        tracing::error!(
                            "Codex permissions approval failed for item_id={}: {err}",
                            params.item_id
                        );
                        err
                    })?;
                let response = PermissionsRequestApprovalResponse {
                    permissions: self.granted_permissions_for_status(&status, params.permissions),
                    scope: PermissionGrantScope::Turn,
                    strict_auto_review: None,
                };
                send_server_response(peer, request_id, response).await
            }
            ServerRequest::DynamicToolCall { request_id, params } => {
                tracing::warn!(
                    "dynamic tool call not supported (namespace={:?}, tool={})",
                    params.namespace,
                    params.tool
                );
                let response = DynamicToolCallResponse {
                    content_items: vec![DynamicToolCallOutputContentItem::InputText {
                        text: format!(
                            "Dynamic tool calls are not supported by this executor: {}",
                            params.tool
                        ),
                    }],
                    success: false,
                };
                send_server_response(peer, request_id, response).await
            }
            ServerRequest::ChatgptAuthTokensRefresh {
                request_id,
                params: _,
            } => {
                let auth = self.get_auth_status().await?;
                let token = auth.auth_token.ok_or_else(|| {
                    ExecutorApprovalError::request_failed("missing auth token for refresh")
                })?;
                let response = ChatgptAuthTokensRefreshResponse {
                    access_token: token,
                    chatgpt_account_id: "unknown".to_string(),
                    chatgpt_plan_type: None,
                };
                send_server_response(peer, request_id, response).await
            }
        }
    }

    async fn request_tool_approval(
        &self,
        tool_name: &str,
        tool_input: Value,
        tool_call_id: &str,
    ) -> Result<ApprovalStatus, ExecutorError> {
        if self.auto_approve {
            return Ok(ApprovalStatus::Approved);
        }
        let approval_service = self
            .approvals
            .as_ref()
            .ok_or(ExecutorApprovalError::ServiceUnavailable)?;

        Ok(approval_service
            .request_tool_approval(tool_name, tool_input, tool_call_id, self.cancel.clone())
            .await?)
    }

    pub async fn register_session(&self, thread_id: &str) -> Result<(), ExecutorError> {
        {
            let mut guard = self.thread_id.lock().await;
            guard.replace(thread_id.to_string());
        }
        self.flush_pending_feedback().await;
        Ok(())
    }

    async fn send_message<M>(&self, message: &M) -> Result<(), ExecutorError>
    where
        M: Serialize + Sync,
    {
        self.rpc().send(message).await
    }

    async fn send_request<R>(&self, request: ClientRequest, label: &str) -> Result<R, ExecutorError>
    where
        R: DeserializeOwned + std::fmt::Debug,
    {
        let request_id = request_id(&request);
        self.rpc()
            .request(request_id, &request, label, self.cancel.clone())
            .await
    }

    fn next_request_id(&self) -> RequestId {
        self.rpc().next_request_id()
    }

    fn command_approval_decision(
        &self,
        status: &ApprovalStatus,
    ) -> (CommandExecutionApprovalDecision, Option<String>) {
        if self.auto_approve {
            return (CommandExecutionApprovalDecision::AcceptForSession, None);
        }

        match status {
            ApprovalStatus::Approved => (CommandExecutionApprovalDecision::Accept, None),
            ApprovalStatus::Denied { reason } => {
                let feedback = reason
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                if feedback.is_some() {
                    (CommandExecutionApprovalDecision::Decline, feedback)
                } else {
                    (CommandExecutionApprovalDecision::Decline, None)
                }
            }
            ApprovalStatus::TimedOut => (CommandExecutionApprovalDecision::Decline, None),
            ApprovalStatus::Pending => (CommandExecutionApprovalDecision::Decline, None),
        }
    }

    fn file_change_approval_decision(
        &self,
        status: &ApprovalStatus,
    ) -> (FileChangeApprovalDecision, Option<String>) {
        if self.auto_approve {
            return (FileChangeApprovalDecision::AcceptForSession, None);
        }

        match status {
            ApprovalStatus::Approved => (FileChangeApprovalDecision::Accept, None),
            ApprovalStatus::Denied { reason } => {
                let feedback = reason
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                (FileChangeApprovalDecision::Decline, feedback)
            }
            ApprovalStatus::TimedOut => (FileChangeApprovalDecision::Decline, None),
            ApprovalStatus::Pending => (FileChangeApprovalDecision::Decline, None),
        }
    }

    fn granted_permissions_for_status(
        &self,
        status: &ApprovalStatus,
        requested: RequestPermissionProfile,
    ) -> GrantedPermissionProfile {
        match status {
            ApprovalStatus::Approved => GrantedPermissionProfile {
                network: requested.network,
                file_system: requested.file_system,
            },
            _ => GrantedPermissionProfile {
                network: None,
                file_system: None,
            },
        }
    }

    fn review_decision(&self, status: &ApprovalStatus) -> (codex_protocol::protocol::ReviewDecision, Option<String>) {
        if self.auto_approve {
            return (codex_protocol::protocol::ReviewDecision::ApprovedForSession, None);
        }

        match status {
            ApprovalStatus::Approved => (codex_protocol::protocol::ReviewDecision::Approved, None),
            ApprovalStatus::Denied { reason } => {
                let feedback = reason
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                if feedback.is_some() {
                    (codex_protocol::protocol::ReviewDecision::Abort, feedback)
                } else {
                    (codex_protocol::protocol::ReviewDecision::Denied, None)
                }
            }
            ApprovalStatus::TimedOut => (codex_protocol::protocol::ReviewDecision::Denied, None),
            ApprovalStatus::Pending => (codex_protocol::protocol::ReviewDecision::Denied, None),
        }
    }

    async fn enqueue_feedback(&self, message: String) {
        if message.trim().is_empty() {
            return;
        }
        let mut guard = self.pending_feedback.lock().await;
        guard.push_back(message);
    }

    async fn flush_pending_feedback(&self) {
        let messages: Vec<String> = {
            let mut guard = self.pending_feedback.lock().await;
            guard.drain(..).collect()
        };

        if messages.is_empty() {
            return;
        }

        let Some(thread_id) = self.thread_id.lock().await.clone() else {
            tracing::warn!(
                "pending Codex feedback but thread id unavailable; dropping {} messages",
                messages.len()
            );
            return;
        };

        for message in messages {
            let trimmed = message.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.spawn_turn_start(thread_id.clone(), format!("User feedback: {trimmed}"));
        }
    }

    fn spawn_turn_start(&self, thread_id: String, message: String) {
        let peer = self.rpc().clone();
        let cancel = self.cancel.clone();
        let request = ClientRequest::TurnStart {
            request_id: peer.next_request_id(),
            params: TurnStartParams {
                thread_id,
                input: vec![UserInput::Text {
                    text: message,
                    text_elements: vec![],
                }],
                responsesapi_client_metadata: None,
                environments: None,
                cwd: None,
                approval_policy: None,
                approvals_reviewer: None,
                sandbox_policy: None,
                permission_profile: None,
                model: None,
                service_tier: None,
                effort: None,
                summary: None,
                personality: None,
                output_schema: None,
                collaboration_mode: None,
            },
        };
        tokio::spawn(async move {
            if let Err(err) = peer
                .request::<TurnStartResponse, _>(
                    request_id(&request),
                    &request,
                    "turn/start",
                    cancel,
                )
                .await
            {
                tracing::error!("failed to start turn: {err}");
            }
        });
    }
}

#[async_trait]
impl JsonRpcCallbacks for AppServerClient {
    async fn on_request(
        &self,
        peer: &JsonRpcPeer,
        raw: &str,
        request: JSONRPCRequest,
    ) -> Result<(), ExecutorError> {
        self.log_writer.log_raw(raw).await?;
        match ServerRequest::try_from(request.clone()) {
            Ok(server_request) => self.handle_server_request(peer, server_request).await,
            Err(err) => {
                tracing::debug!("Unhandled server request `{}`: {err}", request.method);
                let response = JSONRPCResponse {
                    id: request.id,
                    result: Value::Null,
                };
                peer.send(&response).await
            }
        }
    }

    async fn on_response(
        &self,
        _peer: &JsonRpcPeer,
        raw: &str,
        _response: &JSONRPCResponse,
    ) -> Result<(), ExecutorError> {
        self.log_writer.log_raw(raw).await
    }

    async fn on_error(
        &self,
        _peer: &JsonRpcPeer,
        raw: &str,
        _error: &JSONRPCError,
    ) -> Result<(), ExecutorError> {
        self.log_writer.log_raw(raw).await
    }

    async fn on_notification(
        &self,
        _peer: &JsonRpcPeer,
        raw: &str,
        notification: JSONRPCNotification,
    ) -> Result<bool, ExecutorError> {
        let raw = Cow::Borrowed(raw);
        self.log_writer.log_raw(&raw).await?;

        let method = notification.method.as_str();
        let is_turn_completed = method == "turn/completed";

        if method.ends_with("turn_aborted") {
            tracing::debug!("codex turn aborted; flushing feedback queue");
            self.flush_pending_feedback().await;
            return Ok(false);
        }

        let has_finished = is_turn_completed
            || method
                .strip_prefix("codex/event/")
                .is_some_and(|suffix| suffix == "task_complete" || suffix == "turn_complete");

        if has_finished
            && self.commit_reminder
            && !self.commit_reminder_sent.swap(true, Ordering::SeqCst)
            && let status = self.repo_context.check_uncommitted_changes().await
            && !status.is_empty()
            && let Some(thread_id) = self.thread_id.lock().await.clone()
        {
            let prompt = format!("{}\n{}", self.commit_reminder_prompt, status);
            self.spawn_turn_start(thread_id, prompt);
            return Ok(false);
        }

        Ok(has_finished)
    }

    async fn on_non_json(&self, raw: &str) -> Result<(), ExecutorError> {
        self.log_writer.log_raw(raw).await?;
        Ok(())
    }
}

async fn send_server_response<T>(
    peer: &JsonRpcPeer,
    request_id: RequestId,
    response: T,
) -> Result<(), ExecutorError>
where
    T: Serialize,
{
    let payload = JSONRPCResponse {
        id: request_id,
        result: serde_json::to_value(response)
            .map_err(|err| ExecutorError::Io(io::Error::other(err.to_string())))?,
    };

    peer.send(&payload).await
}

fn request_id(request: &ClientRequest) -> RequestId {
    match request {
        ClientRequest::Initialize { request_id, .. }
        | ClientRequest::ThreadStart { request_id, .. }
        | ClientRequest::GetAuthStatus { request_id, .. }
        | ClientRequest::ThreadResume { request_id, .. }
        | ClientRequest::TurnStart { request_id, .. }
        | ClientRequest::ReviewStart { request_id, .. }
        | ClientRequest::McpServerStatusList { request_id, .. } => request_id.clone(),
        _ => unreachable!("request_id called for unsupported request variant"),
    }
}

#[derive(Clone)]
pub struct LogWriter {
    writer: Arc<Mutex<BufWriter<Box<dyn AsyncWrite + Send + Unpin>>>>,
}

impl LogWriter {
    pub fn new(writer: impl AsyncWrite + Send + Unpin + 'static) -> Self {
        Self {
            writer: Arc::new(Mutex::new(BufWriter::new(Box::new(writer)))),
        }
    }

    pub async fn log_raw(&self, raw: &str) -> Result<(), ExecutorError> {
        let mut guard = self.writer.lock().await;
        guard
            .write_all(raw.as_bytes())
            .await
            .map_err(ExecutorError::Io)?;
        guard.write_all(b"\n").await.map_err(ExecutorError::Io)?;
        guard.flush().await.map_err(ExecutorError::Io)?;
        Ok(())
    }
}
