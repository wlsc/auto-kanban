import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

/**
 * Per-attempt UI state for the "Start fresh session" flow.
 *
 * The user arms the flag from the context-usage banner (or actions menu);
 * the next follow-up send consumes it and clears it. This lives at the
 * `TaskAttemptPanel` level so the banner and the follow-up composer can
 * share the same boolean without prop-drilling.
 */
interface FreshSessionContextValue {
  pending: boolean;
  arm: () => void;
  disarm: () => void;
}

const FreshSessionContext = createContext<FreshSessionContextValue | null>(
  null
);

export function FreshSessionProvider({ children }: { children: ReactNode }) {
  const [pending, setPending] = useState(false);

  const arm = useCallback(() => setPending(true), []);
  const disarm = useCallback(() => setPending(false), []);

  const value = useMemo(
    () => ({ pending, arm, disarm }),
    [pending, arm, disarm]
  );

  return (
    <FreshSessionContext.Provider value={value}>
      {children}
    </FreshSessionContext.Provider>
  );
}

export function useFreshSession(): FreshSessionContextValue {
  const ctx = useContext(FreshSessionContext);
  if (!ctx) {
    // Render outside a provider is harmless: callers that don't have it
    // simply behave as if the flag is never armed.
    return { pending: false, arm: () => {}, disarm: () => {} };
  }
  return ctx;
}
