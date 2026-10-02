PRAGMA journal_mode=DELETE;
PRAGMA synchronous=FULL;
CREATE TABLE resources (
 id INTEGER PRIMARY KEY, resource_id TEXT UNIQUE NOT NULL, resource_name TEXT UNIQUE NOT NULL,
 label TEXT NOT NULL, domains TEXT NOT NULL, capabilities TEXT NOT NULL, source_addons TEXT NOT NULL,
 record_file TEXT NOT NULL, source_file TEXT NOT NULL, node_count INTEGER NOT NULL, fact_count INTEGER NOT NULL
);
CREATE TABLE nodes (
 id INTEGER PRIMARY KEY, resource INTEGER NOT NULL REFERENCES resources(id), node_id TEXT NOT NULL,
 ordinal INTEGER NOT NULL, class_name TEXT NOT NULL, instance_name TEXT NOT NULL, view TEXT NOT NULL,
 native_instance_id TEXT, identity_kind TEXT NOT NULL, UNIQUE(resource,node_id)
);
CREATE TABLE fields (
 id INTEGER PRIMARY KEY, class_name TEXT NOT NULL, property TEXT NOT NULL, native_type TEXT NOT NULL,
 effective_count INTEGER NOT NULL DEFAULT 0, ancestor_count INTEGER NOT NULL DEFAULT 0,
 resource_count INTEGER NOT NULL DEFAULT 0, UNIQUE(class_name,property,native_type)
);
CREATE TABLE occurrences (
 node INTEGER NOT NULL REFERENCES nodes(id), field INTEGER NOT NULL REFERENCES fields(id),
 status TEXT NOT NULL, origin TEXT NOT NULL, unit TEXT, PRIMARY KEY(node,field)
) WITHOUT ROWID;
CREATE TABLE aliases (field INTEGER NOT NULL, capability TEXT NOT NULL, alias TEXT NOT NULL,
 PRIMARY KEY(field,capability,alias)) WITHOUT ROWID;
CREATE TABLE capabilities (resource INTEGER NOT NULL, node INTEGER NOT NULL, capability TEXT NOT NULL,
 PRIMARY KEY(resource,node,capability)) WITHOUT ROWID;
CREATE TABLE edges (node INTEGER NOT NULL, target INTEGER NOT NULL, relationship TEXT NOT NULL,
 property TEXT NOT NULL, ordinal INTEGER NOT NULL);
CREATE TABLE resource_references (id INTEGER PRIMARY KEY, resource INTEGER NOT NULL, node INTEGER NOT NULL,
 property TEXT NOT NULL, target INTEGER, target_resource_name TEXT NOT NULL, kind TEXT NOT NULL, method TEXT NOT NULL);
CREATE TABLE summary (key TEXT PRIMARY KEY, value TEXT NOT NULL);
