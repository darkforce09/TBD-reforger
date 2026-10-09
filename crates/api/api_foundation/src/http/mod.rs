//! Request-shape primitives shared by every feature surface: the query and path parameters,
//! parsing rules and required-field rule that are the same whichever resource a route serves.

pub mod pagination;
pub mod path_parameters;
pub mod required_text_field;
