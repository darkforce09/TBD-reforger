**Status:** live

# Account pages documentation

The feature documentation of the three pages that act on the viewer's own session: sign-in, the
Discord sign-in callback and the account settings with the Arma identity link. Developers and AI
agents read it before changing sign-in or the settings page.

## Contents

```text
documentation_v2/website/frontend/pages/account/
└── account_pages.md  the feature doc: sign-in, the dev login, the callback, settings and the API
```

## How it works

One feature doc covers the area, because the three pages form one flow: `/login` hands the browser
to the Discord sign-in the [API](/documentation_v2/glossary.md#api) runs, the API redirects to
`/auth/callback` with the session in the URL fragment, and `/settings` reads and changes the
signed-in account. The doc follows the
[feature doc template](/documentation_v2/standards/templates/feature_doc.md) and links the three
in-code page READMEs for the routes, the calls and every interface text. No visual reference set
exists for these pages, so the folder holds no `visual_references/`.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Sign-in | `/login`, `LoginPage` | "TBD Reforger", "Sign in with Discord" | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md#sign-in) |
| Sign-in callback | `/auth/callback`, `AuthCallbackPage` | "Completing sign-in…" or "Sign-in failed" | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md#sign-in-callback) |
| Account settings | `/settings`, `SettingsPage` | Settings | [account_pages.md](/documentation_v2/website/frontend/pages/account/account_pages.md#settings) |

A new account page gets a section in `account_pages.md` and a row in the table; the doc splits
into one file per page, each with a line in Contents, once it passes 500 lines.

## Code

- [Account pages](/apps/website/frontend/src/v2/pages/account/) — the three route components the
  feature doc describes.
- [Session and access](/apps/website/frontend/src/v2/core/auth/) — the session store, its
  persistence and the [role](/documentation_v2/glossary.md#role) ladder the pages read and write.
- [Identity and access domain](/apps/website/api_v2/src/identity_and_access/) — the Discord
  sign-in, the [dev login](/documentation_v2/glossary.md#dev-login), the profile and the Arma
  link routes.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the page code, the identity and access handlers
  and the ticket registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the [frontend documentation](/documentation_v2/website/frontend/README.md) route table
  and the [page areas](/documentation_v2/website/frontend/pages/README.md) index; the in-code
  READMEs of the account pages and of the session code, which link the feature doc.
- Rules: `account_pages.md` keeps its name, which those links use; it stays within 500 lines; no
  document here holds a screenshot of the built UI.

## Related documentation

- [App layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
  — the frame that renders sign-in bare and the top bar that links the account pages.
- [Local development](/documentation_v2/runbooks/local_development.md) — the dev login and the
  Discord round trip on a workstation.
