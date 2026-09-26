/**
 * @file TBD_MissionVariantStruct.c
 * @brief The named-variant registry row and the variant-gate mirror of slot and vehicle rows.
 *
 * Role: the typed form of `variants[]`, and the root the variant filter's second pass reads slot
 * and vehicle gates into.  Position: filled by `TBD_MissionLoader`'s parse and by
 * `TBD_MissionVariantSources.ParseVariantGateSkeleton`; read by `TBD_MissionVariantFilter`.
 * State: none; plain data.  Invariants: `default` is an Enforce keyword and has no field, so the
 * per-row default flag is read off the raw JSON, index-aligned with the typed `variants[]`; the
 * skeleton is index-aligned with the typed `slots[]` and `vehicles[]`.
 */

//! One named variant from the top-level `variants[]` registry.
//! @contract mission.schema.json#/$defs/variant
class TBD_MissionVariantStruct
{
	string id;    //!< JSON `id`: stable variant key that gated rows name in `variantId`; schema-required.
	string label; //!< JSON `label`: optional display name; empty when absent.
}

//! The `variantId` of one `slots[]` or `vehicles[]` row; every other key of the row is ignored.
//! @contract mission.schema.json#/$defs/slot
//! @contract mission.schema.json#/$defs/vehicle
class TBD_VariantRowRefStruct
{
	string variantId; //!< JSON `variantId`: variant gate; empty = unconditional row.
}

//! Root of the slot and vehicle gate pass: only those two arrays are declared.
//! @contract mission.schema.json#/
class TBD_VariantGateSkeletonStruct
{
	ref array<ref TBD_VariantRowRefStruct> slots;    //!< JSON `slots`: one gate per slot row.
	ref array<ref TBD_VariantRowRefStruct> vehicles; //!< JSON `vehicles`: one gate per vehicle row.
}
