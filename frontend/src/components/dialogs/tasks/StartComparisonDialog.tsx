import { useState, useCallback, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import RepoBranchSelector from '@/components/tasks/RepoBranchSelector';
import { ExecutorProfileSelector } from '@/components/settings';
import { useCreateComparisonTask } from '@/hooks/useComparisons';
import { useRepoBranchSelection, useProjectRepos } from '@/hooks';
import { useUserSystem } from '@/components/ConfigProvider';
import { attemptsApi } from '@/lib/api';
import { workspaceSummaryKeys } from '@/components/ui-new/hooks/useWorkspaces';
import { paths } from '@/lib/paths';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import type { Task, ExecutorProfileId } from 'shared/types';

export interface StartComparisonDialogProps {
  projectId: string;
  workspaceIds: string[];
  taskTitles: string[];
  onSuccess?: (task: Task) => void;
}

const StartComparisonDialogImpl =
  NiceModal.create<StartComparisonDialogProps>(
    ({ projectId, workspaceIds, taskTitles, onSuccess }) => {
      const modal = useModal();
      const navigate = useNavigate();
      const queryClient = useQueryClient();
      const { t } = useTranslation(['tasks', 'common']);
      const { profiles, config } = useUserSystem();
      const createComparisonTask = useCreateComparisonTask();

      const [additionalPrompt, setAdditionalPrompt] = useState('');
      const [title, setTitle] = useState('');
      const [isSubmitting, setIsSubmitting] = useState(false);
      const [error, setError] = useState<string | null>(null);

      const [userSelectedProfile, setUserSelectedProfile] =
        useState<ExecutorProfileId | null>(null);

      const defaultProfile: ExecutorProfileId | null = useMemo(
        () => config?.executor_profile ?? null,
        [config?.executor_profile]
      );

      const effectiveProfile = userSelectedProfile ?? defaultProfile;

      const { data: projectRepos = [], isLoading: isLoadingRepos } =
        useProjectRepos(projectId, { enabled: modal.visible });

      const {
        configs: repoBranchConfigs,
        isLoading: isLoadingBranches,
        setRepoBranch,
        getWorkspaceRepoInputs,
        reset: resetBranchSelection,
      } = useRepoBranchSelection({
        repos: projectRepos,
        enabled: modal.visible && projectRepos.length > 0,
      });

      useEffect(() => {
        if (!modal.visible) {
          setUserSelectedProfile(null);
          resetBranchSelection();
          setAdditionalPrompt('');
          setTitle('');
          setError(null);
        }
      }, [modal.visible, resetBranchSelection]);

      const allBranchesSelected = repoBranchConfigs.every(
        (c) => c.targetBranch !== null
      );

      const isLoadingInitial = isLoadingRepos || isLoadingBranches;

      const canSubmit = Boolean(
        effectiveProfile &&
          allBranchesSelected &&
          projectRepos.length > 0 &&
          !isSubmitting &&
          !isLoadingInitial &&
          workspaceIds.length >= 2
      );

      const handleSubmit = useCallback(async () => {
        if (
          !effectiveProfile ||
          !allBranchesSelected ||
          projectRepos.length === 0
        )
          return;

        setIsSubmitting(true);
        setError(null);

        try {
          // Step 1: Create comparison task
          const task = await createComparisonTask.mutateAsync({
            project_id: projectId,
            workspace_ids: workspaceIds,
            title: title || null,
            additional_prompt: additionalPrompt || null,
          });

          // Step 2: Create attempt (starts execution)
          const repos = getWorkspaceRepoInputs();
          const attempt = await attemptsApi.create({
            task_id: task.id,
            executor_profile_id: effectiveProfile,
            repos,
          });

          // Invalidate caches
          queryClient.invalidateQueries({
            queryKey: ['taskAttempts', task.id],
          });
          queryClient.invalidateQueries({
            queryKey: workspaceSummaryKeys.all,
          });

          onSuccess?.(task);
          modal.hide();

          // Step 3: Navigate to the attempt
          navigate(
            paths.attempt(projectId, task.id, attempt.id)
          );
        } catch (err) {
          console.error(
            'Failed to create comparison task:',
            err
          );
          setError(
            'Failed to create comparison task. Please try again.'
          );
        } finally {
          setIsSubmitting(false);
        }
      }, [
        projectId,
        workspaceIds,
        title,
        additionalPrompt,
        effectiveProfile,
        allBranchesSelected,
        projectRepos.length,
        createComparisonTask,
        getWorkspaceRepoInputs,
        queryClient,
        onSuccess,
        modal,
        navigate,
      ]);

      const handleOpenChange = (open: boolean) => {
        if (!open) modal.hide();
      };

      return (
        <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
          <DialogContent className="sm:max-w-[520px]">
            <DialogHeader>
              <DialogTitle>Compare Solutions</DialogTitle>
              <DialogDescription>
                Create a comparison task for{' '}
                {workspaceIds.length} solutions and start it
                with your chosen agent.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="space-y-2">
                <Label className="text-sm font-medium">
                  Solutions to compare
                </Label>
                <div className="text-sm text-muted-foreground bg-muted/50 rounded-md p-3 max-h-32 overflow-y-auto space-y-1">
                  {taskTitles.map((taskTitle, i) => (
                    <div key={workspaceIds[i]}>
                      <span className="font-medium">
                        {String.fromCharCode(65 + i)}.
                      </span>{' '}
                      {taskTitle}
                    </div>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <Label
                  htmlFor="comparison-title"
                  className="text-sm font-medium"
                >
                  Title (optional)
                </Label>
                <input
                  id="comparison-title"
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="e.g., Authentication approach comparison"
                  className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>

              <div className="space-y-2">
                <Label
                  htmlFor="comparison-prompt"
                  className="text-sm font-medium"
                >
                  Additional instructions (optional)
                </Label>
                <Textarea
                  id="comparison-prompt"
                  value={additionalPrompt}
                  onChange={(e) =>
                    setAdditionalPrompt(e.target.value)
                  }
                  placeholder="Add specific criteria for comparing the solutions..."
                  className="min-h-[80px] resize-none"
                />
              </div>

              {profiles && (
                <div className="space-y-2">
                  <ExecutorProfileSelector
                    profiles={profiles}
                    selectedProfile={effectiveProfile}
                    onProfileSelect={setUserSelectedProfile}
                    showLabel={true}
                  />
                </div>
              )}

              <RepoBranchSelector
                configs={repoBranchConfigs}
                onBranchChange={setRepoBranch}
                isLoading={isLoadingBranches}
                className="space-y-2"
              />

              {error && (
                <div className="text-sm text-destructive">
                  {error}
                </div>
              )}
            </div>

            <DialogFooter className="sm:!justify-between">
              <Button
                variant="outline"
                onClick={() => modal.hide()}
                disabled={isSubmitting}
              >
                {t('common:buttons.cancel')}
              </Button>
              <Button
                onClick={handleSubmit}
                disabled={!canSubmit}
              >
                {isSubmitting
                  ? 'Starting...'
                  : 'Start Comparison'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      );
    }
  );

export const StartComparisonDialog =
  defineModal<StartComparisonDialogProps, void>(
    StartComparisonDialogImpl
  );
