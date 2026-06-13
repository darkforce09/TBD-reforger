import { NavLink } from "react-router-dom";
import { getNavigation } from "../config/navigation";
import "./Sidebar.css";

type Props = {
  isAuthenticated: boolean;
  isAdmin: boolean;
  open: boolean;
  onClose: () => void;
};

export function Sidebar({ isAuthenticated, isAdmin, open, onClose }: Props) {
  const groups = getNavigation(isAuthenticated, isAdmin);

  return (
    <>
      {open && <div className="sidebar-overlay" onClick={onClose} />}
      <aside className={`sidebar ${open ? "sidebar--open" : ""}`}>
        <nav className="sidebar-nav">
          {groups.map((group) => (
            <div key={group.label} className="sidebar-group">
              <p className="sidebar-group-label">{group.label}</p>
              <ul className="sidebar-list">
                {group.items.map((item) => (
                  <li key={item.to}>
                    <NavLink
                      to={item.to}
                      end={item.to === "/"}
                      className={({ isActive }) =>
                        isActive ? "sidebar-link active" : "sidebar-link"
                      }
                      onClick={onClose}
                    >
                      {item.label}
                    </NavLink>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </aside>
    </>
  );
}
