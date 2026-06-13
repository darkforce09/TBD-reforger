export type PageSummary = { slug: string; title: string };
export type PageSection = {
  id: string;
  sectionKey: string;
  heading: string;
  content: string;
  sortOrder: number;
};
export type Page = {
  slug: string;
  title: string;
  published: boolean;
  updatedAt?: string;
  sections: PageSection[];
};

export type User = {
  id: string;
  discordId: string;
  username: string;
  avatarUrl: string;
  createdAt: string;
};

export type AuthMe = { user: User; isAdmin: boolean };

export type EventSummary = {
  id: string;
  title: string;
  slug: string;
  mapName: string;
  startsAt: string;
  endsAt?: string;
  status: string;
  maxPlayers?: number;
  signupsOpen: boolean;
  registeredCount: number;
};

export type Event = EventSummary & {
  description: string;
  published: boolean;
  waitlistCount?: number;
  userRegistration?: Registration;
  createdAt: string;
  updatedAt?: string;
};

export type Announcement = {
  id: string;
  title: string;
  body: string;
  pinned: boolean;
  published: boolean;
  publishedAt?: string;
  createdAt: string;
  updatedAt?: string;
};

export type Registration = {
  id: string;
  eventId: string;
  userId: string;
  status: "registered" | "waitlist" | "cancelled";
  signedUpAt: string;
  cancelledAt?: string;
  user?: User;
  event?: EventSummary;
};

export type UpsertSectionInput = {
  id?: string;
  sectionKey: string;
  heading: string;
  content: string;
  sortOrder: number;
};

export type UpdatePageInput = { title?: string; published?: boolean };

export type CreateEventInput = {
  title: string;
  slug: string;
  description: string;
  mapName: string;
  startsAt: string;
  endsAt?: string;
  status: string;
  maxPlayers?: number;
  signupsOpen: boolean;
  published: boolean;
};

export type UpdateEventInput = Partial<CreateEventInput>;

export type CreateAnnouncementInput = {
  title: string;
  body: string;
  pinned: boolean;
  published: boolean;
};

export type UpdateAnnouncementInput = Partial<CreateAnnouncementInput>;

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    credentials: "include",
    headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
    ...init,
  });
  if (!res.ok) {
    let message = res.statusText;
    try {
      const body = await res.json();
      if (body?.error) message = body.error;
    } catch { /* ignore */ }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const api = {
  listPages: () => request<PageSummary[]>("/api/pages"),
  getPage: (slug: string) => request<Page>(`/api/pages/${slug}`),
  getAdminPage: (slug: string) => request<Page>(`/api/admin/pages/${slug}`),
  updatePage: (slug: string, input: UpdatePageInput) =>
    request<Page>(`/api/admin/pages/${slug}`, { method: "PUT", body: JSON.stringify(input) }),
  upsertSections: (slug: string, sections: UpsertSectionInput[]) =>
    request<Page>(`/api/admin/pages/${slug}/sections`, { method: "PUT", body: JSON.stringify(sections) }),
  deleteSection: (id: string) =>
    request<void>(`/api/admin/sections/${id}`, { method: "DELETE" }),

  listEvents: (opts?: { upcoming?: boolean; past?: boolean }) => {
    const params = new URLSearchParams();
    if (opts?.upcoming) params.set("upcoming", "true");
    if (opts?.past) params.set("past", "true");
    const q = params.toString();
    return request<EventSummary[]>(`/api/events${q ? `?${q}` : ""}`);
  },
  getNextEvent: () => request<EventSummary | null>("/api/events/next"),
  getEvent: (slug: string) => request<Event>(`/api/events/${slug}`),
  getRoster: (slug: string) => request<Registration[]>(`/api/events/${slug}/roster`),
  register: (slug: string) =>
    request<Registration>(`/api/events/${slug}/register`, { method: "POST" }),
  cancelRegistration: (slug: string) =>
    request<void>(`/api/events/${slug}/register`, { method: "DELETE" }),
  myRegistrations: () => request<Registration[]>("/api/me/registrations"),

  listAnnouncements: (limit?: number) =>
    request<Announcement[]>(`/api/announcements${limit ? `?limit=${limit}` : ""}`),

  me: () => request<AuthMe>("/api/auth/me"),
  logout: () => request<void>("/api/auth/logout", { method: "POST" }),

  adminListEvents: () => request<Event[]>("/api/admin/events"),
  adminCreateEvent: (input: CreateEventInput) =>
    request<Event>("/api/admin/events", { method: "POST", body: JSON.stringify(input) }),
  adminUpdateEvent: (id: string, input: UpdateEventInput) =>
    request<Event>(`/api/admin/events/${id}`, { method: "PUT", body: JSON.stringify(input) }),
  adminDeleteEvent: (id: string) =>
    request<void>(`/api/admin/events/${id}`, { method: "DELETE" }),
  adminListRegistrations: (eventId: string) =>
    request<Registration[]>(`/api/admin/events/${eventId}/registrations`),
  adminUpdateRegistration: (id: string, status: string) =>
    request<Registration>(`/api/admin/registrations/${id}`, {
      method: "PUT",
      body: JSON.stringify({ status }),
    }),
  adminDeleteRegistration: (id: string) =>
    request<void>(`/api/admin/registrations/${id}`, { method: "DELETE" }),

  adminListAnnouncements: () => request<Announcement[]>("/api/admin/announcements"),
  adminCreateAnnouncement: (input: CreateAnnouncementInput) =>
    request<Announcement>("/api/admin/announcements", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  adminUpdateAnnouncement: (id: string, input: UpdateAnnouncementInput) =>
    request<Announcement>(`/api/admin/announcements/${id}`, {
      method: "PUT",
      body: JSON.stringify(input),
    }),
  adminDeleteAnnouncement: (id: string) =>
    request<void>(`/api/admin/announcements/${id}`, { method: "DELETE" }),
};

export function discordLoginUrl(returnTo?: string) {
  const base = "/auth/discord";
  if (!returnTo) return base;
  return `${base}?returnTo=${encodeURIComponent(returnTo)}`;
}
