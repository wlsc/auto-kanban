import { useState, useEffect, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { healthApi } from '@/lib/api';

export type ConnectionStatus = 'checking' | 'disconnected' | 'connected';

export function useServerConnection() {
  const [status, setStatus] = useState<ConnectionStatus>('checking');
  const [wasDisconnected, setWasDisconnected] = useState(false);
  const hasEverConnected = useRef(false);

  const { isSuccess, isError } = useQuery({
    queryKey: ['server', 'health'],
    queryFn: async () => {
      const ok = await healthApi.check();
      if (!ok) throw new Error('Health check failed');
      return ok;
    },
    refetchInterval: status !== 'connected' ? 2000 : false,
    retry: false,
    staleTime: 0,
  });

  useEffect(() => {
    if (isSuccess) {
      hasEverConnected.current = true;
      setStatus('connected');
    }
  }, [isSuccess]);

  useEffect(() => {
    if (isError) {
      setStatus('disconnected');
      setWasDisconnected(true);
    }
  }, [isError]);

  // If still checking after a grace period, treat as disconnected
  // so the banner appears instead of a blank screen
  useEffect(() => {
    if (hasEverConnected.current) return;

    const timer = setTimeout(() => {
      if (!hasEverConnected.current) {
        setStatus((prev) => {
          if (prev === 'checking') {
            setWasDisconnected(true);
            return 'disconnected';
          }
          return prev;
        });
      }
    }, 3000);

    return () => clearTimeout(timer);
  }, []);

  return { status, wasDisconnected };
}
