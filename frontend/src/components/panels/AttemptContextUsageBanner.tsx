import { useEntries, useTokenUsage } from '@/contexts/EntriesContext';
import { useFreshSession } from '@/contexts/FreshSessionContext';
import { ContextUsageGauge } from '@/components/ui-new/primitives/ContextUsageGauge';
import { Button } from '@/components/ui/button';
import { ClearContextConfirmDialog } from '@/components/dialogs/attempts/ClearContextConfirmDialog';
import { useAttemptExecution } from '@/hooks/useAttemptExecution';
import { useTranslation } from 'react-i18next';

interface AttemptContextUsageBannerProps {
  workspaceId?: string;
  taskId?: string;
}

export function AttemptContextUsageBanner({
  workspaceId,
  taskId,
}: AttemptContextUsageBannerProps) {
  const { t } = useTranslation('tasks');
  const tokenUsageInfo = useTokenUsage();
  const { entries } = useEntries();
  const { pending, arm } = useFreshSession();
  const { isAttemptRunning } = useAttemptExecution(workspaceId, taskId);

  if (!tokenUsageInfo) return null;

  const percentage = Math.min(
    100,
    (tokenUsageInfo.total_tokens / tokenUsageInfo.model_context_window) * 100
  );

  const formatTokens = (n: number) => {
    if (n >= 1_000_000) {
      const m = n / 1_000_000;
      return m % 1 === 0 ? `${m}M` : `${m.toFixed(1)}M`;
    }
    if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
    return n.toString();
  };

  // Only offer the action when there's actually prior context to clear and
  // when no agent process is currently running for this attempt.
  const canArm = entries.length > 0 && !isAttemptRunning && !pending;

  const handleClick = async () => {
    try {
      const result = await ClearContextConfirmDialog.show({});
      if (result === 'confirmed') {
        arm();
      }
    } catch {
      // User dismissed without confirming — no-op.
    }
  };

  return (
    <div className="shrink-0 flex items-center gap-2 px-4 py-1.5 border-b bg-muted/50 text-xs text-muted-foreground">
      <ContextUsageGauge tokenUsageInfo={tokenUsageInfo} />
      <span>
        {t('contextBanner.usage', {
          used: formatTokens(tokenUsageInfo.total_tokens),
          total: formatTokens(tokenUsageInfo.model_context_window),
          percent: Math.round(percentage),
          defaultValue: 'Context: {{used}} / {{total}} ({{percent}}%)',
        })}
      </span>
      <div className="ml-auto flex items-center gap-2">
        {pending && (
          <span className="text-amber-600 dark:text-amber-400">
            {t('contextBanner.pendingFresh', {
              defaultValue: 'Next message will start a fresh session',
            })}
          </span>
        )}
        {canArm && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs"
            onClick={handleClick}
          >
            {t('contextBanner.startFresh', {
              defaultValue: 'Start fresh session',
            })}
          </Button>
        )}
      </div>
    </div>
  );
}
