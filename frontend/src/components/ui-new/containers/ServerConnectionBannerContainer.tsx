import { useState, useEffect } from 'react';
import { useServerConnectionContext } from '@/contexts/ServerConnectionContext';
import { ServerConnectionBanner } from '../primitives/ServerConnectionBanner';

export function ServerConnectionBannerContainer() {
  const ctx = useServerConnectionContext();
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (ctx?.status !== 'connected') return;

    const timer = setTimeout(() => {
      setDismissed(true);
    }, 1500);

    return () => clearTimeout(timer);
  }, [ctx?.status]);

  if (!ctx || !ctx.wasDisconnected || dismissed) {
    return null;
  }

  return <ServerConnectionBanner status={ctx.status === 'connected' ? 'connected' : 'disconnected'} />;
}
