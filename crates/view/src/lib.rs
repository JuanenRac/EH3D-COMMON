//! What the machine-control programs of the suite show, and nothing that draws it: the scene of a job on the work table (every hole by
//! drill size, the routing paths, the progress, the plan view and the isometric view of the machine fitted to the table), the themes of
//! the interface (the colour type is the one of `eh3d-common`), the standard views (from above, front, side and isometric) and the words of the
//! interface in the eight languages of the project, and the settings of the server (the six boxes, the state of the link, the login and the errors by
//! code in the eight languages) as a model with no window plus, with the feature `gui`, a panel of egui over it.

pub mod clip;
pub mod crash;
pub mod scene;
pub mod server_panel;
pub mod server_texts;
pub mod texts;
pub mod theme;
pub mod views;

#[cfg(feature = "gui")]
pub mod server_gui;
