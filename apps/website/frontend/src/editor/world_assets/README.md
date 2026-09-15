# Graphics host adapter

This module supplies current editor preferences to graphics-engine and registers its render
context with owner cleanup. All asset loading, residency, geometry, and upload work lives in
`website-graphics-engine`. Registration compares both Rc handles so stale cleanup cannot clear
a newer mount.
