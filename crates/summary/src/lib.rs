//! The check before a job starts on a machine that moves: the dry run of the exact plan (how far each axis goes, how long it takes, whether it
//! leaves the course of the machine) and the summary a person reads, in the languages of the suite.
//!
//! It knows nothing of the machine protocol: the program that plans the job feeds the steps it will send to the [`dryrun::Tracker`] and gets the
//! measure back, so the drilling program, the routing program, the modern interface and the server say the same thing.

#![forbid(unsafe_code)]

pub mod dryrun;
pub mod summary;
