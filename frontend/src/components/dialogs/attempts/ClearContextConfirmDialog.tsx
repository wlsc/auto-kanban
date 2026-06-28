import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal, type ConfirmResult, type NoProps } from '@/lib/modals';
import { useTranslation } from 'react-i18next';

const ClearContextConfirmDialogImpl = NiceModal.create<NoProps>(() => {
  const modal = useModal();
  const { t } = useTranslation('tasks');

  const handleConfirm = () => {
    modal.resolve('confirmed' as ConfirmResult);
    modal.hide();
  };

  const handleCancel = () => {
    modal.resolve('canceled' as ConfirmResult);
    modal.hide();
  };

  return (
    <Dialog
      open={modal.visible}
      onOpenChange={(open) => !open && handleCancel()}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {t('clearContextDialog.title', {
              defaultValue: 'Start a fresh agent session?',
            })}
          </DialogTitle>
          <DialogDescription>
            {t('clearContextDialog.body', {
              defaultValue:
                "Your next message will start a brand-new session. The agent's working memory for this attempt will be discarded. The conversation history stays visible here, but the agent will no longer reference earlier turns when generating its next response.",
            })}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={handleCancel} autoFocus>
            {t('clearContextDialog.cancel', { defaultValue: 'Cancel' })}
          </Button>
          <Button onClick={handleConfirm}>
            {t('clearContextDialog.confirm', {
              defaultValue: 'Start fresh session',
            })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export const ClearContextConfirmDialog = defineModal<NoProps, ConfirmResult>(
  ClearContextConfirmDialogImpl
);
