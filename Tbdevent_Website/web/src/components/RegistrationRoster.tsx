import type { Registration } from "../api/client";

type Props = { roster: Registration[] };

export function RegistrationRoster({ roster }: Props) {
  if (roster.length === 0) {
    return <p className="text-muted">No players registered yet.</p>;
  }

  return (
    <ul className="roster-list">
      {roster.map((r) => (
        <li key={r.id} className="roster-item">
          {r.user?.avatarUrl && (
            <img src={r.user.avatarUrl} alt="" className="roster-avatar" />
          )}
          <span>{r.user?.username ?? "Unknown"}</span>
        </li>
      ))}
    </ul>
  );
}
