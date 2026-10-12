//! Bounded import of Azeroth Navigator exchange documents (plan sections 7.1, 7.2, 16.4).
//!
//! Three layers share one validation path:
//! - [`anav1`] is the `ANAV1:` text transport grammar (normative, see
//!   `schemas/v1/README.md`).
//! - [`savedvars`] parses the literal subset of a SavedVariables file as data. It
//!   never evaluates Lua and has no Lua dependency.
//! - [`import`] decodes, validates and summarizes a document. Text and file
//!   input, with either transport, converge on [`import::normalize`].
//!
//! [`fixtures`] is the fixture importer stub behind `azeroth-cli validate-fixtures`.
//!
//! The optional [`watcher`] polls one explicitly selected file. Nothing here persists data or computes a
//! storage identity. Imported state is never labelled live. Errors are
//! `AppError`s with static messages; they never contain imported content, paths
//! or excerpts.
pub mod anav1;
pub mod fixtures;
pub mod import;
pub mod savedvars;
pub mod watcher;

mod error;
