//! Role: residency.
//! Position: `spatial/los/world` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::spatial::bvh::surface::SurfaceKind;
use crate::spatial::los::world::coverage_1::PrefabInfo;
use crate::spatial::los::world::coverage_1::PrefabOccluder;
use crate::spatial::los::world::descriptor::Bounds3;
use crate::spatial::los::world::descriptor::PrefabDescriptor;
use crate::spatial::los::world::placed::ChunkOccluder;
use crate::spatial::los::world::state::WorldOccluder;
use crate::streaming::loaders::chunk::WorldChunk;
use crate::world::architecture::compound::instances::instances_from_records;
use crate::world::environment::buildings::prefab::PrefabRow;
use std::collections::HashSet;
use std::sync::Arc;

impl WorldOccluder {
    /// Change the BLAS byte budget (applied at the next `insert_blas`).
    pub fn set_blas_cap_bytes(&mut self, bytes: usize) {
        self.cap_bytes = bytes;
    }
}

impl WorldOccluder {
    /// The catalogue: kinds, labels and the proxy boxes. The proxy is the catalogue's median world-AABB half extents laid out base-anchored around the origin (`x ±hx`, `y 0..2·hz`, `z ±hy`) — right for the ground-pivoted majority, replaced by the descriptor's exact bounds the moment it expands.
    pub fn set_prefabs<'a>(&mut self, rows: impl Iterator<Item = &'a PrefabRow>) {
        for r in rows {
            let Ok(pid) = u16::try_from(r.prefab_id as u64) else {
                continue;
            };
            if r.prefab_id.fract() != 0.0 {
                continue;
            }
            let proxy = match (r.half_x, r.half_y, r.half_z) {
                (Some(hx), Some(hy), Some(hz)) if hx > 0.0 && hy > 0.0 && hz > 0.0 => {
                    Some(Bounds3 {
                        min: [-hx, 0.0, -hy],
                        max: [hx, 2.0 * hz, hy],
                    })
                }
                _ => None,
            };
            let label = r.label.clone().unwrap_or_else(|| {
                r.resource_name
                    .as_deref()
                    .and_then(|s| s.rsplit('/').next())
                    .map(|s| s.trim_end_matches(".et").to_string())
                    .unwrap_or_else(|| format!("pid {pid}"))
            });
            self.info.insert(
                pid,
                PrefabInfo {
                    kind: r.kind.clone(),
                    label,
                    proxy,
                },
            );
        }
        self.dirty.extend(self.chunks.keys().cloned());
    }
}

impl WorldOccluder {
    /// A chunk became resident.
    pub fn insert_chunk(&mut self, id: &str, chunk: &WorldChunk) {
        self.remove_chunk(id);
        let bounds = |pid: u16| self.bounds_of(pid);
        let c = ChunkOccluder::build(id, chunk, &bounds);
        for r in &c.rows {
            *self.placed.entry(r.pid).or_insert(0) += 1;
        }
        self.chunks.insert(id.to_string(), c);
        self.dirty.remove(id);
    }
}

impl WorldOccluder {
    /// A chunk was evicted.
    pub fn remove_chunk(&mut self, id: &str) {
        if let Some(c) = self.chunks.remove(id) {
            for r in &c.rows {
                if let Some(n) = self.placed.get_mut(&r.pid) {
                    *n = n.saturating_sub(1);
                }
            }
        }
        self.dirty.remove(id);
    }
}

impl WorldOccluder {
    /// Mark dirty for.
    pub(crate) fn mark_dirty_for(&mut self, pid: u16) {
        let ids: Vec<String> = self
            .chunks
            .values()
            .filter(|c| c.rows.iter().any(|r| r.pid == pid))
            .map(|c| c.id.clone())
            .collect();
        self.dirty.extend(ids);
    }
}

impl WorldOccluder {
    /// A descriptor arrived. Expands at once when every BLAS it names is already loaded.
    pub fn insert_descriptor(&mut self, d: PrefabDescriptor) {
        let Ok(pid) = u16::try_from(d.prefab_id) else {
            return;
        };
        if !d.blocks {
            self.no_block.insert(pid);
            self.descriptors.remove(&pid);
            self.expanded.remove(&pid);
            self.mark_dirty_for(pid);
            return;
        }
        self.no_block.remove(&pid);
        self.descriptors.insert(pid, Arc::new(d));
        self.mark_dirty_for(pid);
        self.try_expand(pid);
    }
}

impl WorldOccluder {
    /// A BLAS arrived. Every descriptor waiting on it expands; the byte budget is enforced.
    pub fn insert_blas(&mut self, path: &str, sc: Arc<BvhSidecar>) {
        let bytes = sc.verts.len() * 24 + sc.tris.len() * 13 + sc.bvh.node_count() * 32;
        self.blas_bytes.insert(path.to_string(), bytes);
        self.blas.insert(path.to_string(), sc);
        self.blas_last_use.insert(path.to_string(), self.tick);
        let waiting: Vec<u16> = self
            .descriptors
            .iter()
            .filter(|(pid, d)| {
                !self.expanded.contains_key(pid) && d.instances.iter().any(|i| i.blas == path)
            })
            .map(|(pid, _)| *pid)
            .collect();
        for pid in waiting {
            self.try_expand(pid);
        }
        self.enforce_cap();
    }
}

impl WorldOccluder {
    /// Try expand.
    pub(crate) fn try_expand(&mut self, pid: u16) -> bool {
        let Some(d) = self.descriptors.get(&pid).cloned() else {
            return false;
        };
        if d.instances.iter().any(|i| !self.blas.contains_key(&i.blas)) {
            return false;
        }
        let Ok(instances) = instances_from_records(&d.instances, &self.blas) else {
            return false;
        };
        if instances.is_empty() {
            return false;
        }
        let mut bounds: Option<Bounds3> = None;
        let mut has_foliage = false;
        for inst in &instances {
            let (min, max) = inst.world_aabb();
            let b = Bounds3 { min, max };
            bounds = Some(bounds.map_or(b, |u| u.union(b)));
            has_foliage |= inst.blas.kinds.contains(&SurfaceKind::Foliage);
        }
        let blas_paths: Vec<String> = d.blas_paths().iter().map(ToString::to_string).collect();
        self.expanded.insert(
            pid,
            Arc::new(PrefabOccluder {
                pid,
                instances,
                local_bounds: bounds.unwrap_or(Bounds3 {
                    min: [0.0; 3],
                    max: [0.0; 3],
                }),
                has_foliage,
                blas_paths,
            }),
        );
        self.mark_dirty_for(pid);
        true
    }
}

impl WorldOccluder {
    /// Enforce cap.
    pub(crate) fn enforce_cap(&mut self) {
        let total = |s: &Self| s.blas_bytes.values().sum::<usize>();
        while total(self) > self.cap_bytes {
            let needed: HashSet<&str> = self
                .expanded
                .values()
                .filter(|po| self.placed.get(&po.pid).copied().unwrap_or(0) > 0)
                .flat_map(|po| po.blas_paths.iter().map(String::as_str))
                .collect();
            let victim = self
                .blas
                .keys()
                .filter(|p| !needed.contains(p.as_str()))
                .min_by_key(|p| {
                    (
                        self.blas_last_use.get(*p).copied().unwrap_or(0),
                        (*p).clone(),
                    )
                })
                .cloned();
            let Some(victim) = victim else {
                return;
            };
            self.blas.remove(&victim);
            self.blas_bytes.remove(&victim);
            self.blas_last_use.remove(&victim);
            let unexpand: Vec<u16> = self
                .expanded
                .values()
                .filter(|po| po.blas_paths.contains(&victim))
                .map(|po| po.pid)
                .collect();
            for pid in unexpand {
                self.expanded.remove(&pid);
                self.mark_dirty_for(pid);
            }
        }
    }
}

impl WorldOccluder {
    /// Rebuild the boxes + TLAS of every chunk whose geometry changed since the last refresh.
    pub fn refresh(&mut self) {
        let ids: Vec<String> = self.dirty.drain().collect();
        for id in ids {
            let Some(mut c) = self.chunks.remove(&id) else {
                continue;
            };
            let bounds = |pid: u16| self.bounds_of(pid);
            c.rebuild(&bounds);
            self.chunks.insert(id, c);
        }
    }
}
