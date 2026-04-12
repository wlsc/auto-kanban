use axum::{
    Json, Router,
    extract::State,
    response::Json as ResponseJson,
    routing::post,
};
use db::models::{
    session::Session,
    task::{CreateTask, Task},
    workspace::Workspace,
    workspace_repo::WorkspaceRepo,
};
use executors::executors::{SolutionContext, build_comparison_task_description};
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::workspace::WorkspaceError;
use deployment::Deployment;
use services::services::container::ContainerService;

#[derive(Debug, Deserialize, TS)]
pub struct CreateComparisonTaskRequest {
    pub project_id: Uuid,
    pub workspace_ids: Vec<Uuid>,
    pub title: Option<String>,
    pub additional_prompt: Option<String>,
}

pub async fn create_comparison_task(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateComparisonTaskRequest>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let pool = &deployment.db().pool;

    if payload.workspace_ids.len() < 2 {
        return Err(ApiError::Workspace(WorkspaceError::ValidationError(
            "At least 2 workspaces are required for comparison".to_string(),
        )));
    }

    let mut solutions = Vec::new();
    let mut task_titles = Vec::new();
    let mut task_description: Option<String> = None;

    for (i, workspace_id) in payload.workspace_ids.iter().enumerate() {
        let workspace = Workspace::find_by_id(pool, *workspace_id)
            .await?
            .ok_or(ApiError::Workspace(WorkspaceError::ValidationError(
                format!("Workspace {} not found", workspace_id),
            )))?;

        let task = Task::find_by_id(pool, workspace.task_id)
            .await?
            .ok_or(ApiError::Workspace(WorkspaceError::ValidationError(
                format!("Task for workspace {} not found", workspace_id),
            )))?;

        if task.project_id != payload.project_id {
            return Err(ApiError::Workspace(WorkspaceError::ValidationError(
                format!(
                    "Workspace {} does not belong to project {}",
                    workspace_id, payload.project_id
                ),
            )));
        }

        if task_description.is_none() {
            task_description = Some(task.to_prompt());
        }

        // Ensure container exists so the agent can navigate the worktree
        let container_ref = deployment
            .container()
            .ensure_container_exists(&workspace)
            .await?;

        let executor_used = Session::find_latest_by_workspace_id(pool, workspace.id)
            .await?
            .and_then(|s| s.executor);

        let repos =
            WorkspaceRepo::find_repos_with_target_branch_for_workspace(pool, workspace.id).await?;
        let repo_paths: Vec<(String, String)> = repos
            .iter()
            .map(|r| (r.repo.name.clone(), r.target_branch.clone()))
            .collect();

        let agent_summaries = Workspace::fetch_agent_summaries(pool, workspace.id).await?;

        let label = format!("Solution {}", (b'A' + i as u8) as char);

        task_titles.push(task.title.clone());

        solutions.push(SolutionContext {
            label,
            executor_used,
            container_ref,
            repo_paths,
            agent_summaries,
        });
    }

    let description = build_comparison_task_description(
        task_description.as_deref(),
        &solutions,
        payload.additional_prompt.as_deref(),
    );

    let title = payload.title.unwrap_or_else(|| {
        let joined: String = task_titles
            .iter()
            .take(3)
            .map(|t| {
                if t.len() > 40 {
                    format!("{}...", &t[..37])
                } else {
                    t.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" vs ");
        let base = format!("Compare: {joined}");
        if base.len() > 120 {
            format!("{}...", &base[..117])
        } else {
            base
        }
    });

    let task = Task::create(
        pool,
        &CreateTask {
            project_id: payload.project_id,
            title,
            description: Some(description),
            status: None,
            parent_workspace_id: None,
            image_ids: None,
        },
        Uuid::new_v4(),
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(task)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/comparisons/create-task", post(create_comparison_task))
}
