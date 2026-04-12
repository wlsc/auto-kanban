import { CloudSlashIcon, CheckCircleIcon } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { RunningDots } from './RunningDots';

interface ServerConnectionBannerProps {
  status: 'disconnected' | 'connected';
}

export function ServerConnectionBanner({
  status,
}: ServerConnectionBannerProps) {
  const { t } = useTranslation('common');

  if (status === 'connected') {
    return (
      <div className="absolute inset-0 z-50 flex items-center justify-center bg-primary/80 backdrop-blur-sm animate-fade-out [animation-delay:1.2s]">
        <div className="flex flex-col items-center gap-base">
          <CheckCircleIcon className="size-icon-xl text-success" weight="fill" />
          <p className="text-lg text-normal font-ibm-plex-sans">
            {t('serverConnection.connected')}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-primary/80 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-base">
        <CloudSlashIcon className="size-icon-xl text-low" weight="fill" />
        <div className="flex items-center gap-half">
          <p className="text-lg text-normal font-ibm-plex-sans">
            {t('serverConnection.connecting')}
          </p>
          <RunningDots />
        </div>
        <p className="text-sm text-low">
          {t('serverConnection.description')}
        </p>
      </div>
    </div>
  );
}
