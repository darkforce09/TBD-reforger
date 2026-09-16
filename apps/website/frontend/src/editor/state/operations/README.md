# operations

Frontend adapters for editor operations. These modules read Leptos signals, own host interaction state, call the headless operations in `website-map-engine`'s `data::store::operations`, and refresh the editor after successful mutations. CRDT transaction logic lives there too, not here. (That crate was `website-mission-core` until T-0xx Phase 2A folded it into the map engine.)
