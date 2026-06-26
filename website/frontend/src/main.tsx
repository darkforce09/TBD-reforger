import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router-dom'
import { Toaster } from 'sonner'
import { router } from '@/router'
import './index.css'

// Dev-only: veto Vite's HMR full-page reload while the mission editor is mounted, so an
// alt-tab WS reconnect doesn't cold-boot the Y.Doc (T-062.2). Diagnostics included.
if (import.meta.env.DEV) import('@/dev/viteReloadGuard')

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false },
  },
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster theme="dark" position="top-right" richColors />
    </QueryClientProvider>
  </StrictMode>,
)
