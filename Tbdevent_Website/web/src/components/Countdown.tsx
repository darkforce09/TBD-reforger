import { useEffect, useState } from "react";

type Props = { target: string };

function getParts(target: Date) {
  const diff = target.getTime() - Date.now();
  if (diff <= 0) return null;
  const days = Math.floor(diff / 86400000);
  const hours = Math.floor((diff % 86400000) / 3600000);
  const minutes = Math.floor((diff % 3600000) / 60000);
  return { days, hours, minutes };
}

export function Countdown({ target }: Props) {
  const [parts, setParts] = useState(() => getParts(new Date(target)));

  useEffect(() => {
    const id = setInterval(() => setParts(getParts(new Date(target))), 60000);
    return () => clearInterval(id);
  }, [target]);

  if (!parts) return <span className="countdown countdown--past">Event started</span>;

  return (
    <span className="countdown">
      {parts.days > 0 && `${parts.days}d `}
      {parts.hours}h {parts.minutes}m
    </span>
  );
}
