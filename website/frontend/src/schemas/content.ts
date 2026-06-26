import { z } from 'zod'

export const announcementSchema = z.object({
  title: z.string().min(3),
  tag: z.enum(['Event', 'Modpack Update', 'Important']),
  body: z.string().min(10),
  push_discord: z.boolean().default(false),
})

export type AnnouncementFormValues = z.infer<typeof announcementSchema>
