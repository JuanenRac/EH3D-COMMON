//! The panels and the factory workspaces proposed for each kind of program of the suite (3D printing, circuit boards, mechanical design, milling,
//! pick and place, laser), so that every program of a sector starts with the arrangement people of that sector expect, and the names and titles
//! are the same in all of them. A program takes `panels()` and `workspaces()` of its sector, adds or drops what it needs, and gives them to the
//! frame.

use crate::core::PanelDef;
use crate::geom::Side;
use crate::model::{Layout, Node};
use crate::texts::Text8;
use crate::workspaces::Workspace;

macro_rules! titles {
    ($($name:ident => [$($t:expr),+ $(,)?]),+ $(,)?) => {
        $(
            #[doc = concat!("The title «", stringify!($name), "» in eight languages.")]
            pub const $name: Text8 = Text8([$($t),+]);
        )+
    };
}

/// Titles of the panels the sectors share, each in eight languages (English, Spanish, Italian, German, French, Portuguese of Portugal, Chinese,
/// Japanese).
pub mod titles {
    use super::Text8;
    titles! {
        PROPERTIES => ["Properties", "Propiedades", "Proprietà", "Eigenschaften", "Propriétés", "Propriedades", "属性", "プロパティ"],
        LAYERS => ["Layers", "Capas", "Livelli", "Ebenen", "Calques", "Camadas", "图层", "レイヤー"],
        CONSOLE => ["Console", "Consola", "Console", "Konsole", "Console", "Consola", "控制台", "コンソール"],
        JOG => ["Jog", "Movimiento manual", "Movimento manuale", "Handverfahren", "Déplacement manuel", "Movimento manual", "手动移动", "ジョグ"],
        JOB => ["Job", "Trabajo", "Lavoro", "Auftrag", "Travail", "Trabalho", "作业", "ジョブ"],
        VIEW_3D => ["3D view", "Vista 3D", "Vista 3D", "3D-Ansicht", "Vue 3D", "Vista 3D", "3D 视图", "3Dビュー"],
        VIEWER => ["Viewer", "Visor", "Visualizzatore", "Betrachter", "Visionneuse", "Visualizador", "查看器", "ビューア"],
        FILES => ["Files", "Archivos", "File", "Dateien", "Fichiers", "Ficheiros", "文件", "ファイル"],
        TOOL_TABLE => ["Tool table", "Tabla de herramientas", "Tabella utensili", "Werkzeugtabelle", "Table d'outils", "Tabela de ferramentas", "刀具表", "工具テーブル"],
        PARTS => ["Parts", "Piezas", "Componenti", "Teile", "Pièces", "Peças", "零件", "部品"],
        FEEDERS => ["Feeders", "Alimentadores", "Alimentatori", "Zuführungen", "Alimentateurs", "Alimentadores", "供料器", "フィーダー"],
        BOARD_VIEW => ["Board view", "Vista de la placa", "Vista della scheda", "Platinenansicht", "Vue de la carte", "Vista da placa", "电路板视图", "基板ビュー"],
        DRC => ["Design rule check", "Comprobación de reglas", "Controllo delle regole", "Entwurfsregelprüfung", "Vérification des règles", "Verificação de regras", "设计规则检查", "デザインルールチェック"],
        MODEL_TREE => ["Model tree", "Árbol del modelo", "Albero del modello", "Modellbaum", "Arbre du modèle", "Árvore do modelo", "模型树", "モデルツリー"],
        PRINT_SETTINGS => ["Print settings", "Ajustes de impresión", "Impostazioni di stampa", "Druckeinstellungen", "Réglages d'impression", "Definições de impressão", "打印设置", "印刷設定"],
        CUT_SETTINGS => ["Cut settings", "Ajustes de corte", "Impostazioni di taglio", "Schnitteinstellungen", "Réglages de coupe", "Definições de corte", "切割设置", "カット設定"],
        MATERIALS => ["Materials", "Materiales", "Materiali", "Materialien", "Matériaux", "Materiais", "材料", "材料"],
        GCODE => ["G-code", "G-code", "G-code", "G-Code", "G-code", "G-code", "G 代码", "Gコード"],
        PRINTER => ["Printer", "Impresora", "Stampante", "Drucker", "Imprimante", "Impressora", "打印机", "プリンター"],
        POSITION => ["Position", "Posición", "Posizione", "Position", "Position", "Posição", "位置", "位置"],
        CAMERA => ["Camera", "Cámara", "Fotocamera", "Kamera", "Caméra", "Câmara", "摄像头", "カメラ"],
        LIBRARY => ["Library", "Biblioteca", "Libreria", "Bibliothek", "Bibliothèque", "Biblioteca", "库", "ライブラリ"],
        OBJECTS => ["Objects", "Objetos", "Oggetti", "Objekte", "Objets", "Objetos", "对象", "オブジェクト"],
        REPORT => ["Report", "Informe", "Rapporto", "Bericht", "Rapport", "Relatório", "报告", "レポート"],
        COMPONENTS => ["Components", "Componentes", "Componenti", "Bauteile", "Composants", "Componentes", "元件", "コンポーネント"],
        NETS => ["Nets", "Redes", "Reti", "Netze", "Réseaux", "Redes", "网络", "ネット"],
        MACHINE => ["Machine", "Máquina", "Macchina", "Maschine", "Machine", "Máquina", "机器", "機械"],
        TASKS => ["Tasks", "Tareas", "Attività", "Aufgaben", "Tâches", "Tarefas", "任务", "タスク"],
        PREVIEW => ["Layer preview", "Vista previa de capas", "Anteprima dei livelli", "Schichtvorschau", "Aperçu des couches", "Pré-visualização das camadas", "分层预览", "レイヤープレビュー"],
    }
}

/// The names of the standard workspaces in eight languages (the name saved in the settings stays the English one).
pub fn workspace_title(name: &str) -> Option<Text8> {
    Some(match name {
        "Prepare" => Text8([
            "Prepare",
            "Preparar",
            "Prepara",
            "Vorbereiten",
            "Préparer",
            "Preparar",
            "准备",
            "準備",
        ]),
        "Review" => Text8([
            "Review", "Revisar", "Verifica", "Prüfen", "Réviser", "Rever", "检查", "確認",
        ]),
        "Compact" => Text8([
            "Compact",
            "Compacto",
            "Compatto",
            "Kompakt",
            "Compact",
            "Compacto",
            "紧凑",
            "コンパクト",
        ]),
        "Edit" => Text8([
            "Edit",
            "Editar",
            "Modifica",
            "Bearbeiten",
            "Édition",
            "Editar",
            "编辑",
            "編集",
        ]),
        "Run" => Text8([
            "Run",
            "Ejecutar",
            "Esegui",
            "Ausführen",
            "Exécuter",
            "Executar",
            "运行",
            "実行",
        ]),
        "Setup" => Text8([
            "Setup",
            "Configurar",
            "Configura",
            "Einrichten",
            "Configurer",
            "Configurar",
            "配置",
            "セットアップ",
        ]),
        "Design" => Text8([
            "Design",
            "Diseñar",
            "Progetta",
            "Entwerfen",
            "Concevoir",
            "Desenhar",
            "设计",
            "設計",
        ]),
        "Engrave" => Text8([
            "Engrave",
            "Grabar",
            "Incidi",
            "Gravieren",
            "Graver",
            "Gravar",
            "雕刻",
            "彫刻",
        ]),
        "Model" => Text8([
            "Model",
            "Modelar",
            "Modella",
            "Modellieren",
            "Modéliser",
            "Modelar",
            "建模",
            "モデリング",
        ]),
        _ => return None,
    })
}

/// The sectors of the suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sector {
    /// Slicer and printer control.
    Printer3d,
    /// Circuit board editor.
    Pcb,
    /// Mechanical design.
    Cad,
    /// Milling and drilling machine control.
    Cnc,
    /// Pick and place.
    Pnp,
    /// Laser cutting and engraving.
    Laser,
}

fn def(id: &str, title: Text8, icon: &'static str, w: f32, h: f32) -> PanelDef {
    PanelDef::new(id, title).icon(icon).min(w, h)
}

fn ws(name: &str, root: Node) -> Workspace {
    Workspace::new(name, Layout::with_root(root))
}

impl Sector {
    /// Every sector.
    pub const ALL: [Sector; 6] = [
        Sector::Printer3d,
        Sector::Pcb,
        Sector::Cad,
        Sector::Cnc,
        Sector::Pnp,
        Sector::Laser,
    ];

    /// The word the files use.
    pub fn name(self) -> &'static str {
        match self {
            Sector::Printer3d => "printer3d",
            Sector::Pcb => "pcb",
            Sector::Cad => "cad",
            Sector::Cnc => "cnc",
            Sector::Pnp => "pnp",
            Sector::Laser => "laser",
        }
    }

    /// The panels proposed for the sector.
    pub fn panels(self) -> Vec<PanelDef> {
        use titles::*;
        match self {
            Sector::Printer3d => vec![
                def("view3d", VIEW_3D, "3D", 320.0, 240.0),
                def("objects", OBJECTS, "O", 160.0, 120.0),
                def("print_settings", PRINT_SETTINGS, "S", 220.0, 200.0),
                def("materials", MATERIALS, "M", 180.0, 120.0),
                def("preview", PREVIEW, "L", 240.0, 160.0),
                def("gcode", GCODE, "G", 200.0, 120.0),
                def("printer", PRINTER, "P", 200.0, 160.0),
                def("console", CONSOLE, ">", 200.0, 100.0),
            ],
            Sector::Pcb => vec![
                def("board_view", BOARD_VIEW, "B", 320.0, 240.0),
                def("layers", LAYERS, "L", 160.0, 120.0),
                def("properties", PROPERTIES, "P", 200.0, 160.0),
                def("components", COMPONENTS, "C", 180.0, 140.0),
                def("nets", NETS, "N", 160.0, 120.0),
                def("drc", DRC, "D", 220.0, 120.0),
                def("library", LIBRARY, "B", 180.0, 140.0),
                def("console", CONSOLE, ">", 200.0, 100.0),
            ],
            Sector::Cad => vec![
                def("view3d", VIEW_3D, "3D", 320.0, 240.0),
                def("model_tree", MODEL_TREE, "T", 180.0, 160.0),
                def("properties", PROPERTIES, "P", 200.0, 160.0),
                def("tasks", TASKS, "K", 200.0, 140.0),
                def("report", REPORT, "R", 200.0, 100.0),
                def("console", CONSOLE, ">", 200.0, 100.0),
            ],
            Sector::Cnc => vec![
                def("viewer", VIEWER, "V", 320.0, 240.0),
                def("console", CONSOLE, ">", 220.0, 100.0),
                def("jog", JOG, "J", 200.0, 200.0),
                def("job", JOB, "W", 200.0, 140.0),
                def("position", POSITION, "X", 180.0, 120.0),
                def("tool_table", TOOL_TABLE, "T", 200.0, 140.0),
                def("files", FILES, "F", 180.0, 140.0),
            ],
            Sector::Pnp => vec![
                def("board_view", BOARD_VIEW, "B", 320.0, 240.0),
                def("parts", PARTS, "P", 200.0, 140.0),
                def("feeders", FEEDERS, "F", 200.0, 140.0),
                def("job", JOB, "W", 200.0, 140.0),
                def("camera", CAMERA, "C", 220.0, 160.0),
                def("machine", MACHINE, "M", 200.0, 160.0),
                def("console", CONSOLE, ">", 200.0, 100.0),
            ],
            Sector::Laser => vec![
                def("view", VIEWER, "V", 320.0, 240.0),
                def("layers", LAYERS, "L", 160.0, 120.0),
                def("cut_settings", CUT_SETTINGS, "S", 220.0, 180.0),
                def("library", LIBRARY, "B", 180.0, 140.0),
                def("jog", JOG, "J", 200.0, 200.0),
                def("camera", CAMERA, "C", 220.0, 160.0),
                def("job", JOB, "W", 200.0, 140.0),
                def("console", CONSOLE, ">", 200.0, 100.0),
            ],
        }
    }

    /// The factory workspaces proposed for the sector (three each). A panel that a workspace does not name is hidden in it.
    pub fn workspaces(self) -> Vec<Workspace> {
        let t = |ids: &[&str]| Node::tabs(ids);
        match self {
            Sector::Printer3d => vec![
                ws(
                    "Prepare",
                    Node::row(
                        0.22,
                        t(&["print_settings", "materials"]),
                        Node::row(
                            0.76,
                            Node::column(0.78, t(&["view3d"]), t(&["console", "gcode"])),
                            t(&["objects", "printer"]),
                        ),
                    ),
                ),
                ws(
                    "Review",
                    Node::row(
                        0.76,
                        t(&["view3d", "preview"]),
                        Node::column(0.5, t(&["gcode"]), t(&["print_settings"])),
                    ),
                ),
                ws("Compact", t(&["view3d", "print_settings"])),
            ],
            Sector::Pcb => vec![
                ws(
                    "Edit",
                    Node::row(
                        0.18,
                        t(&["components", "nets", "library"]),
                        Node::row(
                            0.76,
                            Node::column(0.8, t(&["board_view"]), t(&["console", "drc"])),
                            t(&["layers", "properties"]),
                        ),
                    ),
                ),
                ws(
                    "Review",
                    Node::row(
                        0.72,
                        Node::column(0.65, t(&["board_view"]), t(&["drc", "console"])),
                        t(&["properties", "nets"]),
                    ),
                ),
                Workspace::new(
                    "Compact",
                    Layout::with_root(t(&["board_view"]))
                        .with_auto_hidden("layers", Side::Left)
                        .with_auto_hidden("properties", Side::Right),
                ),
            ],
            Sector::Cad => vec![
                ws(
                    "Model",
                    Node::row(
                        0.2,
                        t(&["model_tree", "tasks"]),
                        Node::row(
                            0.78,
                            Node::column(0.78, t(&["view3d"]), t(&["console", "report"])),
                            t(&["properties"]),
                        ),
                    ),
                ),
                ws(
                    "Review",
                    Node::row(
                        0.76,
                        t(&["view3d"]),
                        Node::column(0.5, t(&["properties"]), t(&["report"])),
                    ),
                ),
                Workspace::new(
                    "Compact",
                    Layout::with_root(t(&["view3d"]))
                        .with_auto_hidden("model_tree", Side::Left)
                        .with_auto_hidden("properties", Side::Right),
                ),
            ],
            Sector::Cnc => vec![
                ws(
                    "Run",
                    Node::row(
                        0.22,
                        Node::column(0.5, t(&["position"]), t(&["jog"])),
                        Node::row(
                            0.72,
                            Node::column(0.74, t(&["viewer"]), t(&["console"])),
                            t(&["job", "files"]),
                        ),
                    ),
                ),
                ws(
                    "Prepare",
                    Node::row(
                        0.2,
                        t(&["files", "tool_table"]),
                        Node::column(0.8, t(&["viewer"]), t(&["job", "console"])),
                    ),
                ),
                Workspace::new(
                    "Compact",
                    Layout::with_root(t(&["viewer"]))
                        .with_auto_hidden("console", Side::Bottom)
                        .with_auto_hidden("jog", Side::Left),
                ),
            ],
            Sector::Pnp => vec![
                ws(
                    "Setup",
                    Node::row(
                        0.2,
                        t(&["parts", "feeders"]),
                        Node::row(
                            0.78,
                            Node::column(0.78, t(&["board_view", "camera"]), t(&["console"])),
                            t(&["machine", "job"]),
                        ),
                    ),
                ),
                ws(
                    "Run",
                    Node::row(
                        0.7,
                        Node::column(0.7, t(&["board_view"]), t(&["console"])),
                        Node::column(0.5, t(&["job"]), t(&["camera", "machine"])),
                    ),
                ),
                Workspace::new(
                    "Compact",
                    Layout::with_root(t(&["board_view"]))
                        .with_auto_hidden("parts", Side::Left)
                        .with_auto_hidden("feeders", Side::Left),
                ),
            ],
            Sector::Laser => vec![
                ws(
                    "Design",
                    Node::row(
                        0.2,
                        t(&["layers", "library"]),
                        Node::row(
                            0.76,
                            Node::column(0.8, t(&["view"]), t(&["console"])),
                            t(&["cut_settings", "job"]),
                        ),
                    ),
                ),
                ws(
                    "Engrave",
                    Node::row(
                        0.72,
                        Node::column(0.78, t(&["view", "camera"]), t(&["console"])),
                        Node::column(0.5, t(&["job"]), t(&["jog", "cut_settings"])),
                    ),
                ),
                Workspace::new(
                    "Compact",
                    Layout::with_root(t(&["view"]))
                        .with_auto_hidden("layers", Side::Left)
                        .with_auto_hidden("cut_settings", Side::Right),
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DockCore;
    use eh3d_view::texts::Lang;

    #[test]
    fn every_sector_starts_sound_with_three_workspaces() {
        for s in Sector::ALL {
            let c = DockCore::new(s.panels(), s.workspaces());
            assert_eq!(c.workspaces().names().len(), 3, "{}", s.name());
            for w in c.workspaces().factory() {
                let known: Vec<String> = s.panels().into_iter().map(|d| d.id).collect();
                let refs: Vec<&str> = known.iter().map(String::as_str).collect();
                let mut l = w.layout.clone();
                assert!(
                    l.repair(&refs).is_empty(),
                    "{} {}: {:?}",
                    s.name(),
                    w.name,
                    w.layout.validate(&refs)
                );
                assert!(w.layout.root.is_some());
            }
            assert!(c.layout_is_sound());
        }
    }

    #[test]
    fn the_first_workspace_shows_at_least_three_quarters_of_the_panels() {
        for s in Sector::ALL {
            let c = DockCore::new(s.panels(), s.workspaces());
            let shown = s
                .panels()
                .iter()
                .filter(|d| c.layout().is_shown(&d.id))
                .count();
            assert!(
                shown * 4 >= s.panels().len() * 3,
                "{}: only {shown} of {} shown",
                s.name(),
                s.panels().len()
            );
        }
    }

    #[test]
    fn every_panel_is_shown_in_some_workspace_and_names_are_valid_and_unique() {
        for s in Sector::ALL {
            let c = DockCore::new(s.panels(), s.workspaces());
            let mut ids: Vec<String> = s.panels().into_iter().map(|d| d.id).collect();
            assert!(ids.iter().all(|i| crate::model::valid_id(i)));
            let n = ids.len();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), n, "{}", s.name());
            for id in &ids {
                assert!(
                    c.workspaces()
                        .factory()
                        .iter()
                        .any(|w| w.layout.is_shown(id)
                            || w.layout
                                .state_of(id)
                                .is_some_and(|st| st != crate::model::PanelState::Hidden)),
                    "{}: {id} is hidden in all",
                    s.name()
                );
            }
        }
    }

    #[test]
    fn the_sector_proposals_cover_what_the_owner_asked_for() {
        let ids = |s: Sector| -> Vec<String> { s.panels().into_iter().map(|d| d.id).collect() };
        for need in ["view3d", "objects", "print_settings", "gcode", "preview"] {
            assert!(
                ids(Sector::Printer3d).contains(&need.to_string()),
                "3dp {need}"
            );
        }
        for need in ["board_view", "layers", "properties", "drc"] {
            assert!(ids(Sector::Pcb).contains(&need.to_string()), "pcb {need}");
        }
        for need in ["model_tree", "properties", "view3d"] {
            assert!(ids(Sector::Cad).contains(&need.to_string()), "cad {need}");
        }
        for need in ["viewer", "console", "jog", "job"] {
            assert!(ids(Sector::Cnc).contains(&need.to_string()), "cnc {need}");
        }
        for need in ["parts", "feeders", "board_view"] {
            assert!(ids(Sector::Pnp).contains(&need.to_string()), "pnp {need}");
        }
        for need in ["layers", "cut_settings", "view"] {
            assert!(
                ids(Sector::Laser).contains(&need.to_string()),
                "laser {need}"
            );
        }
    }

    #[test]
    fn every_title_and_workspace_name_exists_in_eight_languages() {
        for s in Sector::ALL {
            for d in s.panels() {
                for lang in Lang::ALL {
                    assert!(!d.title.get(lang).is_empty());
                }
                assert!(
                    !d.title.0[6].is_ascii() || d.title.0[6] == d.title.0[0],
                    "{} zh",
                    d.id
                );
                assert!(d.min_w > 0.0 && d.min_h > 0.0);
            }
            for w in s.workspaces() {
                assert!(workspace_title(&w.name).is_some(), "{}", w.name);
            }
        }
        assert!(workspace_title("Mine").is_none());
    }
}
