//! The panel drawn with egui. A program keeps one [`ViewerPanel`], gives it the attachment with
//! `panel.viewer.open(..)` and calls [`ViewerPanel::show`] inside its dock panel.

use egui::{
    Color32, ColorImage, CornerRadius, Pos2, Rect, RichText, ScrollArea, Sense, TextStyle,
    TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::doc::{Block, Preview};
use crate::limits::ViewerLimits;
use crate::pdf::view::{RenderSpec, Zoom};
use crate::picture::{fit, Raster};
use crate::texts::{tr, Lang, Text};
use crate::viewer::{error_text, Action, Content, PdfPane, Viewer, ViewerEvent};

/// The rows of a document laid out at first, and added by the button under them.
const PAGE_OF_ROWS: usize = 300;
/// The longest line of a text file laid out.
const MAX_LINE_CHARS: usize = 2_000;

#[derive(Clone, Copy)]
enum Asked {
    Do(Action),
    Confirm,
    Cancel,
}

#[derive(Default)]
struct Drawing {
    /// What the texture shows: the attachment and the drawing of the page.
    texture: Option<((u64, u64), TextureHandle)>,
    generation: u64,
    rows: usize,
    query: String,
}

impl Drawing {
    fn texture(&mut self, ui: &Ui, key: (u64, u64), raster: &Raster) -> &TextureHandle {
        if self.texture.as_ref().is_some_and(|(k, _)| *k != key) {
            self.texture = None;
        }
        let (_, texture) = self.texture.get_or_insert_with(|| {
            let side = ui.input(|i| i.max_texture_side);
            let side = u32::try_from(side).unwrap_or(u32::MAX);
            let shown = fit(raster, side);
            let image = ColorImage::from_rgba_unmultiplied(
                [shown.width as usize, shown.height as usize],
                &shown.rgba,
            );
            let loaded = ui
                .ctx()
                .load_texture("eh3d_viewer", image, TextureOptions::LINEAR);
            (key, loaded)
        });
        texture
    }
}

/// The panel of the attachments.
pub struct ViewerPanel {
    pub viewer: Viewer,
    drawing: Drawing,
}

fn image(ui: &mut Ui, texture: &TextureHandle, size: Vec2) -> Rect {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().image(
        texture.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    rect
}

fn pdf(ui: &mut Ui, pane: &mut PdfPane, drawing: &mut Drawing, generation: u64, lang: Lang) {
    let Some(view) = pane.view.as_mut() else {
        ui.label(tr(lang, Text::Opening));
        return;
    };
    let mut search = false;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("◀")
            .on_hover_text(tr(lang, Text::PreviousPage))
            .clicked()
        {
            view.previous_page();
        }
        ui.label(format!(
            "{} {} / {}",
            tr(lang, Text::Page),
            view.page() + 1,
            view.pages()
        ));
        if ui
            .button("▶")
            .on_hover_text(tr(lang, Text::NextPage))
            .clicked()
        {
            view.next_page();
        }
        ui.separator();
        if ui
            .button("−")
            .on_hover_text(tr(lang, Text::ZoomOut))
            .clicked()
        {
            view.zoom_out();
        }
        if let Zoom::Percent(percent) = view.zoom {
            ui.label(format!("{percent} %"));
        }
        if ui
            .button("+")
            .on_hover_text(tr(lang, Text::ZoomIn))
            .clicked()
        {
            view.zoom_in();
        }
        if ui.button(tr(lang, Text::FitWidth)).clicked() {
            view.zoom = Zoom::FitWidth;
        }
        ui.separator();
        let field = ui.add(egui::TextEdit::singleline(&mut drawing.query).desired_width(140.0));
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button(tr(lang, Text::Search)).clicked() || entered {
            search = true;
        }
        if !view.hits().is_empty() {
            if ui
                .button("⏶")
                .on_hover_text(tr(lang, Text::PreviousMatch))
                .clicked()
            {
                view.previous_hit();
            }
            if ui
                .button("⏷")
                .on_hover_text(tr(lang, Text::NextMatch))
                .clicked()
            {
                view.next_hit();
            }
            let at = view.current().map_or(0, |c| c + 1);
            ui.label(format!("{at} / {}", view.hits().len()));
        } else if view.searching() {
            ui.label(tr(lang, Text::Searching));
        } else if !view.query().is_empty() {
            ui.label(tr(lang, Text::NoMatches));
        }
    });
    if search {
        let query = drawing.query.clone();
        pane.search(&query);
    }
    if let Some(error) = &pane.page_error {
        ui.label(tr(lang, error_text(error)));
    }
    ScrollArea::both()
        .id_salt("eh3d_viewer_pdf")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let scale = ui.ctx().pixels_per_point();
            let side = ui.input(|i| i.max_texture_side);
            // whole steps of 32 pixels, so dragging the edge of the panel does not redraw at every pixel
            let width = (ui.available_width() * scale).max(64.0) as u32 / 32 * 32;
            pane.want(RenderSpec {
                zoom: Zoom::FitWidth,
                area_width: width,
                pixels_per_point: scale,
                max_side: u32::try_from(side).unwrap_or(u32::MAX),
            });
            let (Some(view), Some(drawn)) = (&pane.view, &pane.drawn) else {
                ui.spinner();
                return;
            };
            if drawn.page != view.page() {
                ui.spinner();
                return;
            }
            let size = Vec2::new(drawn.raster.width as f32, drawn.raster.height as f32) / scale;
            let texture = drawing.texture(ui, (generation, drawn.ticket), &drawn.raster);
            let rect = image(ui, texture, size);
            for (index, hit) in view.hits().iter().enumerate() {
                if hit.page != drawn.page {
                    continue;
                }
                let color = if view.current() == Some(index) {
                    Color32::from_rgba_unmultiplied(255, 120, 0, 110)
                } else {
                    Color32::from_rgba_unmultiplied(255, 220, 0, 70)
                };
                for area in &hit.areas {
                    let from =
                        |x: f32, y: f32| rect.min + Vec2::new(x * rect.width(), y * rect.height());
                    ui.painter().rect_filled(
                        Rect::from_min_max(
                            from(area.left, area.top),
                            from(area.right, area.bottom),
                        ),
                        CornerRadius::same(1),
                        color,
                    );
                }
            }
        });
}

fn document(ui: &mut Ui, preview: &Preview, drawing: &mut Drawing) {
    ScrollArea::both()
        .id_salt("eh3d_viewer_document")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut left = drawing.rows.max(PAGE_OF_ROWS);
            let mut cut = false;
            for (index, block) in preview.blocks.iter().enumerate() {
                if left == 0 {
                    cut = true;
                    break;
                }
                match block {
                    Block::Heading(level, text) => {
                        let size = 22.0 - 2.0 * f32::from((*level).min(5));
                        ui.label(RichText::new(text).size(size).strong());
                    }
                    Block::Paragraph(text) => {
                        ui.label(text);
                    }
                    Block::Item(depth, text) => {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(12.0 * f32::from(*depth));
                            ui.label("•");
                            ui.label(text);
                        });
                    }
                    Block::Code(text) => {
                        ui.monospace(text);
                    }
                    Block::Quote(text) => {
                        ui.label(RichText::new(text).italics());
                    }
                    Block::Rule => {
                        ui.separator();
                    }
                    Block::Table(rows) => {
                        let shown = rows.len().min(left);
                        cut |= shown < rows.len();
                        egui::Grid::new(("eh3d_viewer_table", index))
                            .striped(true)
                            .show(ui, |ui| {
                                for row in &rows[..shown] {
                                    for cell in row {
                                        ui.label(cell);
                                    }
                                    ui.end_row();
                                }
                            });
                        left = left.saturating_sub(shown.saturating_sub(1));
                    }
                }
                left = left.saturating_sub(1);
            }
            if cut && ui.button("▼").clicked() {
                drawing.rows = drawing.rows.max(PAGE_OF_ROWS) + PAGE_OF_ROWS;
            }
        });
}

fn lines(ui: &mut Ui, text: &str) {
    let height = ui.text_style_height(&TextStyle::Monospace);
    let count = text.lines().count();
    ScrollArea::both()
        .id_salt("eh3d_viewer_text")
        .auto_shrink([false, false])
        .show_rows(ui, height, count, |ui, range| {
            for line in text.lines().skip(range.start).take(range.len()) {
                let end = line
                    .char_indices()
                    .nth(MAX_LINE_CHARS)
                    .map_or(line.len(), |(at, _)| at);
                ui.add(
                    egui::Label::new(RichText::new(&line[..end]).monospace())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            }
        });
}

impl ViewerPanel {
    pub fn new(limits: ViewerLimits) -> Self {
        ViewerPanel::with(Viewer::new(limits))
    }

    pub fn with(viewer: Viewer) -> Self {
        ViewerPanel {
            viewer,
            drawing: Drawing::default(),
        }
    }

    /// Draws the panel and gives what the program has to do, if anything.
    pub fn show(&mut self, ui: &mut Ui, lang: Lang) -> Option<ViewerEvent> {
        self.viewer.poll();
        let generation = self.viewer.generation();
        if self.drawing.generation != generation {
            self.drawing = Drawing {
                generation,
                ..Drawing::default()
            };
        }
        let asked = self.head(ui, lang);
        self.body(ui, lang);
        // the answers of the PDF thread are read on the next frames
        if self.viewer.pdf_mut().is_some_and(|pane| pane.busy()) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        match asked? {
            Asked::Do(action) => self.viewer.request(action),
            Asked::Confirm => self.viewer.confirm(),
            Asked::Cancel => {
                self.viewer.cancel();
                None
            }
        }
    }

    fn head(&mut self, ui: &mut Ui, lang: Lang) -> Option<Asked> {
        let viewer = &self.viewer;
        let Some(class) = viewer.classification() else {
            ui.label(tr(lang, Text::Empty));
            return None;
        };
        let mut asked = None;
        ui.strong(&class.name);
        let warning = viewer.warning();
        if let Some(text) = warning {
            let color = ui.visuals().warn_fg_color;
            ui.colored_label(color, tr(lang, text));
        }
        ui.horizontal_wrapped(|ui| {
            let open = egui::Button::new(tr(lang, Text::OpenOutside));
            if ui.add_enabled(class.may_launch(), open).clicked() {
                asked = Some(Asked::Do(Action::OpenOutside));
            }
            if ui.button(tr(lang, Text::SaveCopy)).clicked() {
                asked = Some(Asked::Do(Action::SaveCopy));
            }
            if matches!(viewer.content(), Content::Withheld { has_preview: true })
                && ui.button(tr(lang, Text::ShowPreview)).clicked()
            {
                asked = Some(Asked::Do(Action::Preview));
            }
        });
        if !class.may_launch() {
            ui.label(tr(lang, Text::Refused));
        }
        if viewer.pending().is_some() {
            ui.group(|ui| {
                if let Some(text) = warning {
                    ui.strong(tr(lang, text));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr(lang, Text::Confirm)).clicked() {
                        asked = Some(Asked::Confirm);
                    }
                    if ui.button(tr(lang, Text::Cancel)).clicked() {
                        asked = Some(Asked::Cancel);
                    }
                });
            });
        }
        if let Some(notice) = viewer.notice() {
            ui.label(tr(lang, notice));
        }
        ui.separator();
        asked
    }

    fn body(&mut self, ui: &mut Ui, lang: Lang) {
        let generation = self.drawing.generation;
        let drawing = &mut self.drawing;
        if let Some(pane) = self.viewer.pdf_mut() {
            pdf(ui, pane, drawing, generation, lang);
            return;
        }
        match self.viewer.content() {
            Content::Empty | Content::Withheld { .. } | Content::Pdf(_) => {}
            Content::Outside(reason) => {
                let text = reason.as_ref().map_or(Text::NoPreview, error_text);
                ui.label(tr(lang, text));
            }
            Content::Picture(raster) => {
                let texture = drawing.texture(ui, (generation, 0), raster).clone();
                let size = Vec2::new(raster.width as f32, raster.height as f32);
                ScrollArea::both()
                    .id_salt("eh3d_viewer_picture")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // never larger than it is, and no wider than the panel
                        let scale = (ui.available_width() / size.x).clamp(0.01, 1.0);
                        image(ui, &texture, size * scale);
                    });
            }
            Content::Document { preview, text_only } => {
                if *text_only {
                    ui.weak(tr(lang, Text::TextOnly));
                }
                if preview.truncated {
                    ui.weak(tr(lang, Text::Truncated));
                }
                document(ui, preview, drawing);
            }
            Content::Text { text, truncated } => {
                if *truncated {
                    ui.weak(tr(lang, Text::Truncated));
                }
                lines(ui, text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MemorySource;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn frame(ctx: &egui::Context, panel: &mut ViewerPanel, lang: Lang) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            panel.show(ui, lang);
        });
        out.drop_without_applying_deltas();
    }

    #[test]
    fn every_kind_of_content_is_drawn_in_every_language() {
        let ctx = egui::Context::default();
        let mut panel = ViewerPanel::new(ViewerLimits::default());
        let mut picture = Vec::new();
        image::RgbaImage::from_pixel(3000, 4, image::Rgba([1, 2, 3, 255]))
            .write_to(
                &mut std::io::Cursor::new(&mut picture),
                image::ImageFormat::Png,
            )
            .unwrap();
        let files: [(&str, Vec<u8>); 7] = [
            ("a.txt", "line\nline 日本".repeat(500).into_bytes()),
            (
                "a.md",
                b"# T\n- a\n> b\n---\n```\nc\n```\n|a|b|\n|-|-|\n|1|2|".to_vec(),
            ),
            ("a.csv", "a,b\n".repeat(900).into_bytes()),
            ("a.png", picture),
            ("a.exe", b"MZ\0\0".to_vec()),
            ("a.mp4", b"video".to_vec()),
            ("a.docx", b"not a zip".to_vec()),
        ];
        frame(&ctx, &mut panel, Lang::En);
        for (name, data) in files {
            panel.viewer.open(Arc::new(MemorySource::new(name, data)));
            for lang in Lang::ALL {
                frame(&ctx, &mut panel, lang);
            }
            panel.viewer.request(Action::SaveCopy);
            frame(&ctx, &mut panel, Lang::Es);
        }
    }

    struct Blank;

    impl crate::pdf::PdfDocument for Blank {
        fn page_count(&self) -> u32 {
            2
        }
        fn page_size(&self, _: u32) -> Result<(f32, f32), crate::ViewerError> {
            Ok((200.0, 100.0))
        }
        fn render(&self, _: u32, width: u32, height: u32) -> Result<Raster, crate::ViewerError> {
            Ok(Raster {
                width,
                height,
                rgba: vec![255; width as usize * height as usize * 4],
            })
        }
        fn find(
            &self,
            page: u32,
            _: &str,
            _: usize,
        ) -> Result<Vec<crate::pdf::Hit>, crate::ViewerError> {
            Ok(vec![crate::pdf::Hit {
                page,
                areas: vec![crate::pdf::Area {
                    left: 0.1,
                    top: 0.1,
                    right: 0.2,
                    bottom: 0.2,
                }],
            }])
        }
    }

    #[test]
    fn a_pdf_page_and_its_matches_are_drawn() {
        let ctx = egui::Context::default();
        let viewer = Viewer::new(ViewerLimits::default()).with_pdf_engine(|_| Ok(Box::new(Blank)));
        let mut panel = ViewerPanel::with(viewer);
        panel
            .viewer
            .open(Arc::new(MemorySource::new("a.pdf", b"%PDF".to_vec())));
        let until = Instant::now() + Duration::from_secs(20);
        let mut searched = false;
        loop {
            assert!(Instant::now() < until, "no page was drawn");
            frame(&ctx, &mut panel, Lang::Ja);
            let pane = panel.viewer.pdf_mut().expect("a PDF");
            if pane.drawn.is_some() && !searched {
                pane.search("a");
                searched = true;
            }
            if searched && !pane.busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        frame(&ctx, &mut panel, Lang::Ja);
        let pane = panel.viewer.pdf_mut().unwrap();
        assert_eq!(pane.view.as_ref().unwrap().hits().len(), 2);
        assert!(panel.drawing.texture.is_some());
    }
}
