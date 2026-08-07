//! Error construction helpers.
//!
//! A typed [`TermtvError`] enum will eventually replace these generic
//! `io::Error` helpers. For now they centralize InvalidData construction.

use std::io;

/// Create a generic invalid-data IO error from a static message.
pub(crate) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Create a generic invalid-data IO error from an owned message.
pub(crate) fn invalid_owned(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
