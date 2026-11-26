//! The frame of a professional window for the programs of the suite: panels that can be docked, tabbed, split, floated, folded into an edge,
//! hidden, maximised and locked; named workspaces saved in the settings file; a menu bar declared as data with shortcuts, recent files and the
//! View, Window and Help menus generated from the frame; and every word of it in the eight languages of the project.
//!
//! The crate has two layers. The model has no window in it and is always there: the layout tree and its operations ([`model`]), the geometry
//! of a window ([`geom`], [`arrange`]), the text form of a layout ([`serial`]), the workspaces ([`workspaces`]), the settings lines ([`config`],
//! [`recent`]), the shortcuts ([`shortcuts`]), the menus ([`menu`], [`stdmenus`]), the panels and workspaces proposed for each sector of the
//! suite ([`sectors`]), what the window about a program of the PHOENIX family says ([`about`]), the colours of that family ([`palette`]), the whole state of the frame ([`core`]) and a picture of a layout as SVG ([`preview`]). With the feature `gui` the same state is drawn with
//! egui 0.37 ([`gui`]).

pub mod about;
pub mod arrange;
pub mod config;
pub mod core;
pub mod geom;
pub mod menu;
pub mod model;
pub mod palette;
pub mod preview;
pub mod recent;
pub mod sectors;
pub mod serial;
pub mod shortcuts;
pub mod stdmenus;
pub mod texts;
pub mod workspaces;

#[cfg(feature = "gui")]
pub mod gui;
