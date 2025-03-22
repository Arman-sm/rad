// This crate is the heart of the project.
// It includes everything actually related to the audio itself.
// Here exists code for the compositor which generated audio according to the configuration of its composition state,
// code for several source types (e.g. formatted audio source (mp3, wav, ...), queue, iter...)
// code for many utilities and types used all around the project.

pub mod composition;
pub mod compositor;
pub mod cmp_reg;
pub mod adapter;
pub mod prelude;
pub mod source;