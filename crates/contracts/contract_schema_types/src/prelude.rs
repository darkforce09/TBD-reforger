//! The names most callers import with `use contract_schema_types::prelude::*;`: the domain
//! modules, each holding one module per schema. The schema modules are not flattened, because two
//! schemas may generate types of the same name.

pub use crate::generated::*;
