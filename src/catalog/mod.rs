//! The catalog: DotForge's database of components and applications.
//!
//! The built-in catalog (the `catalog/` folder of the repository) is
//! embedded into the program when it is compiled. Users can add their own
//! entries in `~/.config/dotforge/catalog/`; a user entry with the same ID
//! as a built-in one replaces it.

#[allow(clippy::module_inception)]
pub mod catalog;
pub mod conventions;
pub mod loader;
pub mod model;
pub mod validate;
