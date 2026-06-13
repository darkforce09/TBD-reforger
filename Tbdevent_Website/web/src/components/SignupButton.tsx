import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { Event } from "../api/client";
import { api, discordLoginUrl } from "../api/client";
import { useAuth } from "../hooks/useAuth";

type Props = { event: Event };

export function SignupButton({ event }: Props) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const reg = event.userRegistration;

  const registerMutation = useMutation({
    mutationFn: () => api.register(event.slug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["event", event.slug] });
      queryClient.invalidateQueries({ queryKey: ["my-registrations"] });
    },
  });

  const cancelMutation = useMutation({
    mutationFn: () => api.cancelRegistration(event.slug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["event", event.slug] });
      queryClient.invalidateQueries({ queryKey: ["my-registrations"] });
    },
  });

  if (!event.signupsOpen || event.status !== "published") {
    return <p className="signup-closed">Sign-ups are closed for this event.</p>;
  }

  if (!isAuthenticated) {
    return (
      <a
        className="btn btn-primary"
        href={discordLoginUrl(`/events/${event.slug}`)}
      >
        Login with Discord to sign up
      </a>
    );
  }

  if (reg?.status === "registered") {
    return (
      <div className="signup-status">
        <span className="badge badge--registered">You are registered</span>
        <button
          className="btn btn-danger"
          disabled={cancelMutation.isPending}
          onClick={() => cancelMutation.mutate()}
        >
          {cancelMutation.isPending ? "Cancelling…" : "Cancel registration"}
        </button>
        {cancelMutation.error && (
          <p className="error-text">{(cancelMutation.error as Error).message}</p>
        )}
      </div>
    );
  }

  if (reg?.status === "waitlist") {
    return (
      <div className="signup-status">
        <span className="badge badge--waitlist">You are on the waitlist</span>
        <button
          className="btn btn-danger"
          disabled={cancelMutation.isPending}
          onClick={() => cancelMutation.mutate()}
        >
          Leave waitlist
        </button>
      </div>
    );
  }

  return (
    <div className="signup-status">
      <button
        className="btn btn-primary"
        disabled={registerMutation.isPending}
        onClick={() => registerMutation.mutate()}
      >
        {registerMutation.isPending ? "Signing up…" : "Sign up for event"}
      </button>
      {registerMutation.error && (
        <p className="error-text">{(registerMutation.error as Error).message}</p>
      )}
    </div>
  );
}
