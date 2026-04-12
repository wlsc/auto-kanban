import { useMutation, useQueryClient } from '@tanstack/react-query';
import { comparisonsApi } from '@/lib/api';
import type { Task, CreateComparisonTaskRequest } from 'shared/types';

export const useCreateComparisonTask = () => {
  const queryClient = useQueryClient();
  return useMutation<Task, Error, CreateComparisonTaskRequest>({
    mutationFn: comparisonsApi.createTask,
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['project-tasks', variables.project_id],
      });
    },
  });
};
