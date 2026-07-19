import { InfoIcon } from '@phosphor-icons/react';
import { cn } from '@/lib/utils';

interface ChatSystemMessageProps {
  content: string;
  className?: string;
  expanded?: boolean;
  onToggle?: () => void;
  /** Optional reasoning-effort label rendered as a badge next to the content. */
  effort?: string | null;
}

export function ChatSystemMessage({
  content,
  className,
  expanded,
  onToggle,
  effort,
}: ChatSystemMessageProps) {
  return (
    <div
      className={cn(
        'flex items-start gap-base text-sm text-low cursor-pointer',
        className
      )}
      onClick={onToggle}
      role="button"
    >
      <InfoIcon className="shrink-0 size-icon-base pt-0.5" />
      <span
        className={cn(
          'min-w-0',
          !expanded && 'truncate',
          expanded && 'whitespace-pre-wrap break-all'
        )}
      >
        {content}
      </span>
      {effort && (
        <span className="shrink-0 rounded-sm border border-brand/40 bg-brand/10 px-half text-xs uppercase tracking-wide text-brand">
          {effort}
        </span>
      )}
    </div>
  );
}
