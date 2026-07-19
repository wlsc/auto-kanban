import { Gauge, ArrowDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Label } from '@/components/ui/label';
import { getEffortOptions } from '@/utils/executor';
import type { BaseCodingAgent, EffortLevel } from 'shared/types';

interface EffortSelectorProps {
  executor: BaseCodingAgent | null | undefined;
  selectedEffort: EffortLevel | null;
  onChange: (effort: EffortLevel | null) => void;
  disabled?: boolean;
  className?: string;
  showLabel?: boolean;
}

const AUTO = 'AUTO';

export function EffortSelector({
  executor,
  selectedEffort,
  onChange,
  disabled,
  className = '',
  showLabel = false,
}: EffortSelectorProps) {
  const options = getEffortOptions(executor);

  // Executors without a reasoning-effort concept render nothing.
  if (options.length === 0) return null;

  const label = selectedEffort ? selectedEffort.toUpperCase() : AUTO;

  return (
    <div className="flex-1">
      {showLabel && (
        <Label htmlFor="executor-effort" className="text-sm font-medium">
          Reasoning Effort
        </Label>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className={`w-full justify-between text-xs ${showLabel ? 'mt-1.5' : ''} ${className}`}
            disabled={disabled}
            aria-label="Select reasoning effort"
          >
            <div className="flex items-center gap-1.5 w-full">
              <Gauge className="h-3 w-3" />
              <span className="truncate">{label}</span>
            </div>
            <ArrowDown className="h-3 w-3" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent className="w-60">
          <DropdownMenuItem
            onClick={() => onChange(null)}
            className={selectedEffort === null ? 'bg-accent' : ''}
          >
            {AUTO}
          </DropdownMenuItem>
          {options.map((effort) => (
            <DropdownMenuItem
              key={effort}
              onClick={() => onChange(effort)}
              className={selectedEffort === effort ? 'bg-accent' : ''}
            >
              {effort.toUpperCase()}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
