import { z } from 'zod'

export const eventSchema = z.object({
  title: z.string().min(3, 'Title required'),
  mission_id: z.string().uuid('Select a mission'),
  scheduled_at: z.string().min(1, 'Date and time required'),
  registration_open: z.boolean().default(true),
})

export type EventFormValues = z.infer<typeof eventSchema>
