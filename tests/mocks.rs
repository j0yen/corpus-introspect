//! Mock test entry point — wires in tests/mocks/ac6.rs so it runs under
//! `cargo test`. Without this file, tests/mocks/*.rs would compile to nothing.
//!
//! See autobuilder skill: "Orphaned mock tests false-green" note.

mod ac6;
