# Ticket sync

Regenerates three outputs from the ticket files: the dispatch queue `.ai/tickets/queue.json`, the recommended-next-work block between the roadmap's `ticket-sync:next` markers, and the ticket column of the gap-analysis tables. `cmd_sync` writes them in that order and writes no other file. It skips a roadmap or gap-analysis file that is absent, and a roadmap without its start marker; it refuses a write that would leave the marker block structurally empty, and it rewrites the gap-analysis column only after the tables round-trip byte for byte.
