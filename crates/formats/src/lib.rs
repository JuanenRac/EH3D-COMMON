//! The files that the Bungard CCD programs read and write: the Excellon drill file ([`ncd`]), the HP-GL routing file ([`hpgl`]), the tool
//! tables of the drilling and of the routing program ([`drill_tools`], [`route_tools`]) and the machine configuration ([`config`]). They
//! are read as the machines' own programs read them and written back keeping their layout (the lines that are not understood stay as
//! they were), so that a file edited here still opens in the original program.

pub mod config;
pub mod drill_tools;
pub mod hpgl;
pub mod ncd;
pub mod route_tools;
