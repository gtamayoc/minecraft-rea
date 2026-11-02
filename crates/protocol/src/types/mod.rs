//! Protocol primitive types module.

pub mod position;
pub mod varint;

pub use position::{read_position, read_string, write_position, write_string, Position};
pub use varint::{read_varint, read_varlong, varint_size, write_varint, write_varlong};
