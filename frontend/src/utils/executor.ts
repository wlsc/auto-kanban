import type {
  BaseCodingAgent,
  ExecutorConfigs,
  ExecutorAction,
  ExecutorProfileId,
  ExecutionProcess,
} from 'shared/types';
import { EffortLevel } from 'shared/types';

/**
 * Reasoning-effort options offered per executor, expressed in the shared
 * {@link EffortLevel} scale. The backend clamps values the executor can't
 * represent (e.g. Codex has no `max`, Droid tops out at `high`), so we only
 * surface levels that map to a distinct setting. Executors without a
 * reasoning-effort concept are omitted and render no selector.
 */
const EFFORT_OPTIONS_BY_EXECUTOR: Partial<
  Record<BaseCodingAgent, EffortLevel[]>
> = {
  CLAUDE_CODE: [
    EffortLevel.low,
    EffortLevel.medium,
    EffortLevel.high,
    EffortLevel.xhigh,
    EffortLevel.max,
  ],
  CODEX: [
    EffortLevel.low,
    EffortLevel.medium,
    EffortLevel.high,
    EffortLevel.xhigh,
  ],
  DROID: [EffortLevel.low, EffortLevel.medium, EffortLevel.high],
};

/**
 * Get the reasoning-effort options for an executor, or an empty array when the
 * executor doesn't support configuring effort.
 */
export function getEffortOptions(
  executor: BaseCodingAgent | null | undefined
): EffortLevel[] {
  if (!executor) return [];
  return EFFORT_OPTIONS_BY_EXECUTOR[executor] ?? [];
}

/**
 * Compare two ExecutorProfileIds for equality.
 * Treats null/undefined variant as equivalent to "DEFAULT".
 */
export function areProfilesEqual(
  a: ExecutorProfileId | null | undefined,
  b: ExecutorProfileId | null | undefined
): boolean {
  if (!a || !b) return a === b;
  if (a.executor !== b.executor) return false;
  // Normalize variants: null/undefined -> 'DEFAULT'
  const variantA = a.variant ?? 'DEFAULT';
  const variantB = b.variant ?? 'DEFAULT';
  return variantA === variantB;
}

/**
 * Get variant options for a given executor from profiles.
 * Returns variants sorted: DEFAULT first, then alphabetically.
 */
export function getVariantOptions(
  executor: BaseCodingAgent | null | undefined,
  profiles: ExecutorConfigs['executors'] | null | undefined
): string[] {
  if (!executor || !profiles) return [];
  const executorConfig = profiles[executor];
  if (!executorConfig) return [];

  const variants = Object.keys(executorConfig);
  return variants.sort((a, b) => {
    if (a === 'DEFAULT') return -1;
    if (b === 'DEFAULT') return 1;
    return a.localeCompare(b);
  });
}

/**
 * Extract ExecutorProfileId from an ExecutorAction chain.
 * Traverses the action chain to find the first coding agent request.
 */
export function extractProfileFromAction(
  action: ExecutorAction | null
): ExecutorProfileId | null {
  let curr: ExecutorAction | null = action;
  while (curr) {
    const typ = curr.typ;
    switch (typ.type) {
      case 'CodingAgentInitialRequest':
      case 'CodingAgentFollowUpRequest':
      case 'ReviewRequest':
        return typ.executor_profile_id;
      case 'ScriptRequest':
      default:
        curr = curr.next_action;
        continue;
    }
  }
  return null;
}

/**
 * Get the latest ExecutorProfileId from a list of execution processes.
 * Searches from most recent to oldest.
 */
export function getLatestProfileFromProcesses(
  processes: ExecutionProcess[] | undefined
): ExecutorProfileId | null {
  if (!processes?.length) return null;
  return (
    processes
      .slice()
      .reverse()
      .map((p) => extractProfileFromAction(p.executor_action ?? null))
      .find((pid) => pid !== null) ?? null
  );
}
