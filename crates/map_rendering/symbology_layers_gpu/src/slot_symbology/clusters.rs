//! **Role:** what a camera change does to the slot symbology (the zoom uniform, the cluster gate)
//! and the cluster marker lane fed from the cluster index over the visible rectangle.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; the
//! renderer's camera frame hook calls [`SlotSymbology::camera_changed`], and the slot bind feeds
//! the cluster lane after every rebind.
//! **Signals & state:** the slot bridge's cluster mode, cluster index and the cluster lane.
//! **Invariants:** the cluster index is rebuilt only when the slot count changes; outside cluster
//! mode the cluster lane is empty; a camera change before the atlas is armed does nothing.

use super::view::SlotSymbology;
use map_coordinates::terrain_frames::EVERON_BOUNDS;
use map_draw_lanes::lane_roles::LaneRole;
use overlay_instances::symbols::pack_cluster_instances;

impl SlotSymbology<'_> {
    /// Camera moved: px_to_m + cluster gate re-eval (zoom is engine SoT).
    pub fn camera_changed(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        self.sync_slot_zoom_uniform();
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = overlay_instances::symbols::cluster_mode(n, zoom);
        let mode_changed = cm != self.slot_bridge.last_cluster_mode;
        if mode_changed {
            self.slot_bridge.last_cluster_mode = cm;
            if !self.slot_bridge.drag_active {
                self.rematerialize_slot_lane();
            }
        }

        self.feed_cluster_markers();
    }

    /// Feed cluster markers.
    pub(super) fn feed_cluster_markers(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let n = self.slot_bridge.last_ids.len();
        #[allow(clippy::cast_possible_truncation)]
        if !overlay_instances::symbols::cluster_mode(n as u32, self.zoom()) {
            self.upload_cluster_lane(&[], false);
            return;
        }

        if self.slot_bridge.cluster_index.is_none() || self.slot_bridge.cluster_built_len != n {
            let world: Vec<(f64, f64)> = self
                .slot_bridge
                .last_xy
                .chunks_exact(2)
                .map(|c| (f64::from(c[0]), f64::from(c[1])))
                .collect();
            self.slot_bridge.cluster_index = Some(
                spatial_indexes::point_indexes::cluster::ClusterIndex::build(
                    &world,
                    EVERON_BOUNDS[2],
                    EVERON_BOUNDS[3],
                ),
            );
            self.slot_bridge.cluster_built_len = n;
        }
        let rect = self.camera.visible_world_rect();
        let zoom = self.zoom();
        let (xs, ys, counts) = {
            let idx = self
                .slot_bridge
                .cluster_index
                .as_ref()
                .expect("built above");
            let markers = idx.get_clusters(rect[0], rect[1], rect[2], rect[3], zoom);
            let xs: Vec<f64> = markers.iter().map(|m| m.x).collect();
            let ys: Vec<f64> = markers.iter().map(|m| m.y).collect();
            let counts: Vec<u32> = markers.iter().map(|m| m.count).collect();
            (xs, ys, counts)
        };
        let bytes = pack_cluster_instances(&xs, &ys, &counts);
        self.upload_cluster_lane(&bytes, !bytes.is_empty());
    }

    /// Upload cluster lane.
    pub(super) fn upload_cluster_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Clusters, bytes, visible);
    }
}
