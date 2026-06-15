//! mudshark-core — shared building blocks for mudshark commands.
//!
//! Every command speaks the same dialect: pick an output [`Format`], build a
//! [`json::Json`] value (serialised identically everywhere) or a [`table`],
//! and format byte counts and timestamps with [`bytes`] and [`time`].
//!
//! Dependency-free (std only) so it builds on the distro Rust toolchain. When
//! a newer Rust is available this is the layer that would adopt serde/clap.

pub mod bytes;
pub mod json;
pub mod table;
pub mod time;

mod format;
pub use format::Format;
