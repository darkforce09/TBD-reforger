export type NavItem = {
  to: string;
  label: string;
  requiresAuth?: boolean;
  requiresAdmin?: boolean;
};

export type NavGroup = {
  label: string;
  items: NavItem[];
};

export function getNavigation(isAuthenticated: boolean, isAdmin: boolean): NavGroup[] {
  const groups: NavGroup[] = [
    {
      label: "General",
      items: [
        { to: "/", label: "Home" },
        { to: "/announcements", label: "Announcements" },
        { to: "/events", label: "Events" },
        { to: "/my-events", label: "My Events", requiresAuth: true },
      ],
    },
    {
      label: "Information",
      items: [
        { to: "/rules", label: "Rules" },
        { to: "/compliance", label: "Compliance" },
        { to: "/server", label: "Server Info" },
        { to: "/mods", label: "Mods" },
      ],
    },
  ];

  if (isAdmin) {
    groups.push({
      label: "Staff",
      items: [{ to: "/admin", label: "Admin", requiresAdmin: true }],
    });
  }

  return groups.map((group) => ({
    ...group,
    items: group.items.filter((item) => {
      if (item.requiresAdmin && !isAdmin) return false;
      if (item.requiresAuth && !isAuthenticated) return false;
      return true;
    }),
  }));
}
