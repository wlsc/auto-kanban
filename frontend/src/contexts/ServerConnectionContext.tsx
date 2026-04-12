import { createContext, useContext, type ReactNode } from 'react';
import {
  useServerConnection,
  type ConnectionStatus,
} from '@/hooks/useServerConnection';

export interface ServerConnectionContextValue {
  status: ConnectionStatus;
  wasDisconnected: boolean;
}

const ServerConnectionContext =
  createContext<ServerConnectionContextValue | null>(null);

interface ServerConnectionProviderProps {
  children: ReactNode;
}

export function ServerConnectionProvider({
  children,
}: ServerConnectionProviderProps) {
  const value = useServerConnection();

  return (
    <ServerConnectionContext.Provider value={value}>
      {children}
    </ServerConnectionContext.Provider>
  );
}

/**
 * Returns null if used outside of ServerConnectionProvider (graceful fallback).
 */
export function useServerConnectionContext(): ServerConnectionContextValue | null {
  return useContext(ServerConnectionContext);
}
