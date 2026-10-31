//! The check of the work from a picture of the camera of the drill head (EH3D-NODE-BUNGARD-AUX). After a job the head looks at the board: [`holes::check_holes`] says for each
//! hole of the drill job whether it is there, how wide it is, how far it is from where it should be, whether it is round and whether it is clean; [`milling::check_milling`]
//! measures the channels the router left between copper and copper (their width, how even it is, where they pinch) and [`quality::assess`] says whether the picture itself
//! is good enough to judge anything (light, contrast, focus). A [`profile::Profile`] holds every tolerance and which checks are on, so a shop can make its own analyser
//! (a preset for fine pitch work, another for a quick prototype), and [`report::inspect`] runs what the profile asks and gives one verdict: ok, not ok, or unknown when the
//! picture was too poor to say.
//!
//! Nothing here touches the network or a screen: the picture goes in as grey pixels (from a JPEG or from RGB) and the answer comes out as plain data and as JSON, so the
//! desktop program, the server and the tests read the same words. The coordinates in millimetres are relative to the centre of the picture, x to the right and y up, which is
//! where the tool is when the camera is centred on it; turning the position of a hole of the job into that is the caller's, because it knows where the head is.

pub mod blobs;
pub mod edt;
pub mod holes;
pub mod image;
pub mod milling;
pub mod profile;
pub mod quality;
pub mod report;

pub use holes::{check_holes, ExpectedHole, HoleIssue, HoleResult};
pub use image::Gray;
pub use milling::{check_milling, MillingIssue, MillingResult};
pub use profile::{Polarity, Profile};
pub use quality::{assess, suggest_light_level, ImageQuality, QualityIssue};
pub use report::{inspect, Finding, Report, Verdict};
