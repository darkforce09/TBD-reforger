# Ticket storage

The typed transactional corpus is implemented in `../store.rs`. Compatibility projections and historical formats live in `../registry/`. Live writes use typed operations and surgical corpus persistence; compatibility writers retain their format restrictions.
