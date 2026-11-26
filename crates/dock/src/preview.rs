//! A picture of a layout as SVG, drawn from the model alone: the groups of tabs, the dividers, the floating windows and the folded tabs. It is what
//! the demonstration writes for `--shot`, what the documentation shows, and a way to look at a factory layout without opening a window.

use crate::arrange::{arrange, tab_rows};
use crate::core::PanelDef;
use crate::geom::{Rect, Side};
use crate::model::{Layout, Node};
use eh3d_view::texts::Lang;

const BAR: f32 = 22.0;
const GAP: f32 = 4.0;
const EDGE: f32 = 22.0;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn text_width(s: &str) -> f32 {
    s.chars()
        .map(|c| if c.is_ascii() { 7.0 } else { 13.0 })
        .sum()
}

fn title<'a>(defs: &'a [PanelDef], id: &'a str, lang: Lang) -> &'a str {
    defs.iter()
        .find(|d| d.id == id)
        .map(|d| d.title.get(lang))
        .unwrap_or(id)
}

fn tabs_svg(
    out: &mut String,
    r: Rect,
    ids: &[String],
    active: usize,
    defs: &[PanelDef],
    lang: Lang,
) {
    out.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"#1f2430\" stroke=\"#3a4256\"/>\n",
        r.x, r.y, r.w, r.h
    ));
    let widths: Vec<f32> = ids
        .iter()
        .map(|id| text_width(title(defs, id, lang)) + 16.0)
        .collect();
    let most = ((r.h / 2.0 / BAR) as usize).max(2);
    let rows = tab_rows(&widths, r.w - 8.0, r.w - 8.0, 3.0, active, most);
    let bar_h = BAR * rows.rows as f32;
    out.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{bar_h}\" rx=\"3\" fill=\"#2a3142\"/>\n",
        r.x, r.y, r.w
    ));
    for (i, (id, place)) in ids.iter().zip(&rows.places).enumerate() {
        let Some(place) = place.filter(|p| p.w >= 14.0) else {
            continue;
        };
        let (x, y) = (r.x + 4.0 + place.x, r.y + BAR * place.row as f32);
        let w = place.w;
        let fill = if i == active { "#3b82f6" } else { "#364058" };
        out.push_str(&format!(
            "<rect x=\"{x:.1}\" y=\"{:.1}\" width=\"{w:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"{fill}\"/>\n",
            y + 3.0,
            BAR - 5.0
        ));
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"#e8ecf5\" font-size=\"12\" font-family=\"sans-serif\" clip-path=\"inset(0)\">{}</text>\n",
            x + 8.0,
            y + 16.0,
            esc(title(defs, id, lang))
        ));
    }
    if let Some(a) = ids.get(active) {
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"#8893ab\" font-size=\"11\" font-family=\"sans-serif\">{}</text>\n",
            r.x + 10.0,
            r.y + bar_h + 20.0,
            esc(a)
        ));
    }
}

/// The SVG of the layout in a window of `width` by `height` points, with the titles in `lang`.
pub fn layout_svg(
    layout: &Layout,
    defs: &[PanelDef],
    lang: Lang,
    width: f32,
    height: f32,
) -> String {
    let width = if width.is_finite() {
        width.clamp(200.0, 4000.0)
    } else {
        1200.0
    };
    let height = if height.is_finite() {
        height.clamp(150.0, 3000.0)
    } else {
        700.0
    };
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.0} {height:.0}\">\n<rect width=\"100%\" height=\"100%\" fill=\"#12151c\"/>\n"
    );
    // folded tabs take a strip on their side
    let mut area = Rect::new(0.0, 0.0, width, height);
    for side in Side::ALL {
        if layout.auto_hidden.iter().any(|(_, s)| *s == side) {
            area = match side {
                Side::Left => Rect::new(area.x + EDGE, area.y, area.w - EDGE, area.h),
                Side::Right => Rect::new(area.x, area.y, area.w - EDGE, area.h),
                Side::Top => Rect::new(area.x, area.y + EDGE, area.w, area.h - EDGE),
                Side::Bottom => Rect::new(area.x, area.y, area.w, area.h - EDGE),
            };
        }
    }
    let area = area.shrink(4.0);
    let mins = |id: &str| {
        defs.iter()
            .find(|d| d.id == id)
            .map(|d| (d.min_w, d.min_h))
            .unwrap_or((0.0, 0.0))
    };
    match (&layout.root, &layout.maximized) {
        (Some(root), Some(m)) => {
            let ids = layout
                .group_of(m)
                .map(|g| (g.panels.clone(), g.active))
                .unwrap_or((vec![m.clone()], 0));
            tabs_svg(&mut out, area, &ids.0, ids.1, defs, lang);
            let _ = root;
        }
        (Some(root), None) => {
            let arranged = arrange(root, area, GAP, &mins);
            for leaf in &arranged.leaves {
                if let Some(Node::Tabs(g)) = root.get(&leaf.path) {
                    tabs_svg(&mut out, leaf.rect, &g.panels, g.active, defs, lang);
                }
            }
            for d in &arranged.dividers {
                out.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#53607c\"/>\n",
                    d.rect.x, d.rect.y, d.rect.w, d.rect.h
                ));
            }
        }
        (None, _) => {}
    }
    for side in Side::ALL {
        let mut pos = 6.0;
        for (id, s) in layout.auto_hidden.iter().filter(|(_, s)| *s == side) {
            let t = title(defs, id, lang);
            let len = text_width(t) + 14.0;
            let (x, y, w, h) = match s {
                Side::Left => (0.0, pos, EDGE - 2.0, len),
                Side::Right => (width - EDGE + 2.0, pos, EDGE - 2.0, len),
                Side::Top => (pos, 0.0, len, EDGE - 2.0),
                Side::Bottom => (pos, height - EDGE + 2.0, len, EDGE - 2.0),
            };
            out.push_str(&format!(
                "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" fill=\"#364058\"/>\n"
            ));
            let (tx, ty) = match s {
                Side::Left | Side::Right => (x + 4.0, y + 12.0),
                Side::Top | Side::Bottom => (x + 7.0, y + 14.0),
            };
            out.push_str(&format!(
                "<text x=\"{tx:.1}\" y=\"{ty:.1}\" fill=\"#e8ecf5\" font-size=\"11\" font-family=\"sans-serif\">{}</text>\n",
                esc(&t.chars().take(2).collect::<String>())
            ));
            pos += if matches!(s, Side::Left | Side::Right) {
                h + 4.0
            } else {
                w + 4.0
            };
        }
    }
    for f in &layout.floating {
        let r = Rect::new(
            f.rect.x.min(width - 40.0),
            f.rect.y.min(height - 40.0),
            f.rect.w.min(width),
            f.rect.h.min(height),
        );
        out.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"#000\" opacity=\"0.35\" transform=\"translate(4 4)\"/>\n",
            r.x, r.y, r.w, r.h
        ));
        tabs_svg(&mut out, r, &f.group.panels, f.group.active, defs, lang);
        out.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"none\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>\n",
            r.x, r.y, r.w, r.h
        ));
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DockCore;
    use crate::sectors::Sector;

    fn balanced(svg: &str) -> bool {
        svg.matches('<').count() == svg.matches('>').count()
            && svg.starts_with("<svg")
            && svg.trim_end().ends_with("</svg>")
    }

    #[test]
    fn every_factory_workspace_of_every_sector_draws() {
        for s in Sector::ALL {
            let c = DockCore::new(s.panels(), s.workspaces());
            for w in c.workspaces().factory() {
                for lang in [Lang::En, Lang::Zh, Lang::Ja] {
                    let svg = layout_svg(&w.layout, c.defs(), lang, 1200.0, 700.0);
                    assert!(balanced(&svg), "{} {}", s.name(), w.name);
                    assert!(
                        !svg.contains("NaN") && !svg.contains("inf"),
                        "{} {}",
                        s.name(),
                        w.name
                    );
                }
            }
        }
    }

    #[test]
    fn titles_are_written_and_escaped() {
        let c = DockCore::new(Sector::Pcb.panels(), Sector::Pcb.workspaces());
        let svg = layout_svg(c.layout(), c.defs(), Lang::Es, 1200.0, 700.0);
        assert!(svg.contains("Capas") && svg.contains("Propiedades"));
        assert_eq!(esc("a<b&\"c\">"), "a&lt;b&amp;&quot;c&quot;&gt;");
    }

    #[test]
    fn floating_folded_and_maximised_panels_show() {
        let mut c = DockCore::new(Sector::Cnc.panels(), Sector::Cnc.workspaces());
        c.layout_mut().float("jog", None).unwrap();
        c.layout_mut().auto_hide("console", Side::Bottom).unwrap();
        let svg = layout_svg(c.layout(), c.defs(), Lang::En, 1000.0, 600.0);
        assert!(
            svg.contains("stroke=\"#f59e0b\""),
            "a floating window has its outline"
        );
        assert!(svg.contains("Co"), "the folded tab shows its first letters");
        c.layout_mut().maximize("viewer").unwrap();
        let big = layout_svg(c.layout(), c.defs(), Lang::En, 1000.0, 600.0);
        assert!(balanced(&big));
        assert!(big.matches("fill=\"#1f2430\"").count() < svg.matches("fill=\"#1f2430\"").count());
    }

    #[test]
    fn nonsense_sizes_do_not_break_the_picture() {
        let c = DockCore::new(Sector::Cad.panels(), Sector::Cad.workspaces());
        for (w, h) in [(0.0, 0.0), (f32::NAN, 5.0), (1.0e9, 1.0e9)] {
            let svg = layout_svg(c.layout(), c.defs(), Lang::En, w, h);
            assert!(balanced(&svg));
        }
    }
}
