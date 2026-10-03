// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/audit-log.schema.json — regenerate with: cargo xtask ci schema-codegen

///GET /api/v1/admin/audit-logs/stream (administrator, text/event-stream): the event that opens every stream, `event: ready`, whose id is the start cursor, resume_after. Without Last-Event-ID the stream starts at the tail, the newest publication sequence; with one it resumes after that sequence, and a malformed one answers 400. retained_after is the retained floor: any publication sequence at or below it may be missing. After ready, each published audit line is an unnamed event whose id is its publication sequence and whose data is an AuditLogEntry. A client connects, waits for ready, loads history from the list route and drops stream lines whose id it already holds.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuditStreamReady {
    pub resume_after: u64,
    pub retained_after: u64,
}
