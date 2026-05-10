import { useMemo } from 'react';
import { useQuery, keepPreviousData } from '@tanstack/react-query';
import type { TokenUsageInfo, WorkspaceSummary } from 'shared/types';
import { workspaceSummaryKeys } from '@/components/ui-new/hooks/useWorkspaces';

async function fetchWorkspaceSummaries(): Promise<
  Map<string, WorkspaceSummary>
> {
  const response = await fetch('/api/task-attempts/summary', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ archived: false }),
  });

  if (!response.ok) return new Map();

  const data = await response.json();
  if (!data.success || !data.data?.summaries) return new Map();

  const map = new Map<string, WorkspaceSummary>();
  for (const summary of data.data.summaries) {
    map.set(summary.workspace_id, summary);
  }
  return map;
}

/**
 * Read token usage for multiple workspaces, reactively subscribing to
 * the same workspace-summary query the sidebar uses.
 */
export function useWorkspacesTokenUsage(
  workspaceIds: string[]
): Map<string, TokenUsageInfo> {
  const { data: summaries } = useQuery({
    queryKey: workspaceSummaryKeys.byArchived(false),
    queryFn: fetchWorkspaceSummaries,
    staleTime: 1000,
    refetchInterval: 15000,
    refetchOnWindowFocus: false,
    placeholderData: keepPreviousData,
  });

  return useMemo(() => {
    const result = new Map<string, TokenUsageInfo>();
    if (!summaries || workspaceIds.length === 0) return result;

    for (const id of workspaceIds) {
      const usage = summaries.get(id)?.token_usage;
      if (usage) {
        result.set(id, usage);
      }
    }
    return result;
  }, [summaries, workspaceIds]);
}
