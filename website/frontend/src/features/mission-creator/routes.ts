// Lazy registration for the Mission Creator route. Keeping the React.lazy boundary
// inside the feature module lets Vite code-split the whole editor (Deck.gl, and the
// Yjs/worker bundles added in later phases) so it never loads for users who don't
// open it (Ultra Plan §1.3).

import { lazy } from 'react'

export const MissionEditorPage = lazy(() => import('./MissionCreatorPage'))
