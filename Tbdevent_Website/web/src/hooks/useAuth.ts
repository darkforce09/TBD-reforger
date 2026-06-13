import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";

export function useAuth() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ["auth", "me"],
    queryFn: () => api.me(),
    retry: false,
  });

  const logout = async () => {
    await api.logout();
    await queryClient.invalidateQueries({ queryKey: ["auth", "me"] });
  };

  return {
    user: query.data?.user ?? null,
    isAdmin: query.data?.isAdmin ?? false,
    isLoading: query.isLoading,
    isAuthenticated: !!query.data?.user,
    logout,
    refetch: query.refetch,
  };
}
