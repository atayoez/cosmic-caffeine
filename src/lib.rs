//! Shared types for the cosmic-caffeine applet and settings GUI.
//!
//! The applet (`src/main.rs`) and the standalone settings binary
//! (`src/bin/cosmic-caffeine-settings.rs`) both depend on this lib so
//! the config schema, inhibit wrapper, and Fluent loader live in one
//! place.

pub mod config;
pub mod inhibit;
pub mod localize;

pub const APP_ID: &str = "io.github.atayozcan.CosmicCaffeine";
pub const BIN_NAME: &str = "cosmic-caffeine";
