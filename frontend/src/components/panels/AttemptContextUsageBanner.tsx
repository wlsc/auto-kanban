import { useTokenUsage } from '@/contexts/EntriesContext';
import { ContextUsageGauge } from '@/components/ui-new/primitives/ContextUsageGauge';

export function AttemptContextUsageBanner() {
  const tokenUsageInfo = useTokenUsage();

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

  return (
    <div className="shrink-0 flex items-center gap-2 px-4 py-1.5 border-b bg-muted/50 text-xs text-muted-foreground">
      <ContextUsageGauge tokenUsageInfo={tokenUsageInfo} />
      <span>
        Context: {formatTokens(tokenUsageInfo.total_tokens)} /{' '}
        {formatTokens(tokenUsageInfo.model_context_window)} (
        {Math.round(percentage)}%)
      </span>
    </div>
  );
}
