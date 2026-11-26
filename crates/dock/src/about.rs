//! What the window «About» of a program of the PHOENIX family says: who wrote it, where it lives, its licence, one sentence about what it is and
//! the colour that tells it apart. There is no drawing here, so all of it is tested without a window; the feature `gui` draws it
//! (`gui::about_contents`).

use crate::core::AboutInfo;
use crate::texts::{tr, DockText, Text8};
use eh3d_view::texts::Lang;

/// Who writes the programs.
pub const AUTHOR: &str = "JuanenRac (Electro Hobby 3D)";
/// Where the author is written to.
pub const EMAIL: &str = "electrohobby3d@gmail.com";
/// The page of the author.
pub const AUTHOR_URL: &str = "https://github.com/JuanenRac";
/// The licence of the programs.
pub const LICENSE: &str = "GPL-3.0-or-later";

/// A program of the PHOENIX family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phoenix {
    /// Circuit boards.
    Pcb,
    /// Pick and place.
    Pnp,
    /// Mechanical design.
    Cad,
    /// Milling and drilling machines.
    Cnc,
    /// Laser cutting and engraving.
    Laser,
    /// 3D printing.
    Printer3d,
}

impl Phoenix {
    /// Every program of the family.
    pub const ALL: [Phoenix; 6] = [
        Phoenix::Pcb,
        Phoenix::Pnp,
        Phoenix::Cad,
        Phoenix::Cnc,
        Phoenix::Laser,
        Phoenix::Printer3d,
    ];

    /// The name of the program, which is also the name of its repository.
    pub fn name(self) -> &'static str {
        match self {
            Phoenix::Pcb => "EH3D-PHOENIX-PCB",
            Phoenix::Pnp => "EH3D-PHOENIX-PNP",
            Phoenix::Cad => "EH3D-PHOENIX-CAD",
            Phoenix::Cnc => "EH3D-PHOENIX-CNC",
            Phoenix::Laser => "EH3D-PHOENIX-LASER",
            Phoenix::Printer3d => "EH3D-PHOENIX-3DP",
        }
    }

    /// The accent colour of the program (red, green, blue), the principal tone of [`crate::palette`]: fire, emerald, ice blue, amethyst, crimson and
    /// gold. It is a colour for fills and rings; text takes [`crate::palette::readable`].
    pub fn accent(self) -> [u8; 3] {
        self.tones().main
    }

    /// The address of the repository of the program.
    pub fn repository(self) -> String {
        format!("{AUTHOR_URL}/{}", self.name())
    }

    /// Whether the picture of the program is a round badge (it can fill the ring) or a square one (it is drawn smaller, inside it).
    pub fn round_logo(self) -> bool {
        // the six programs have a phoenix inside a round badge today
        true
    }

    /// One sentence about what the program is, in eight languages.
    pub fn summary(self) -> Text8 {
        match self {
            Phoenix::Pcb => Text8([
                "Design of circuit boards: view, check and edit boards and libraries, and prepare the files to manufacture them.",
                "Diseño de placas de circuito: ver, comprobar y editar placas y librerías, y preparar los archivos para fabricarlas.",
                "Progettazione di circuiti stampati: vedere, controllare e modificare schede e librerie e preparare i file per produrle.",
                "Entwurf von Leiterplatten: Platinen und Bibliotheken ansehen, prüfen und bearbeiten und die Fertigungsdaten erstellen.",
                "Conception de circuits imprimés : voir, vérifier et modifier les cartes et les bibliothèques, et préparer les fichiers de fabrication.",
                "Desenho de placas de circuito: ver, verificar e editar placas e bibliotecas e preparar os ficheiros para o fabrico.",
                "电路板设计：查看、检查和编辑电路板与元件库，并生成制造文件。",
                "基板設計：基板とライブラリの表示・検査・編集と、製造用ファイルの作成を行います。",
            ]),
            Phoenix::Pnp => Text8([
                "Pick and place: prepares the jobs, checks parts and feeders, plans the order of placement and follows the machine.",
                "Pick and place: prepara los trabajos, comprueba componentes y alimentadores, planifica el orden de colocación y sigue la máquina.",
                "Pick and place: prepara i lavori, controlla componenti e alimentatori, pianifica l'ordine di posa e segue la macchina.",
                "Bestückung: bereitet Aufträge vor, prüft Bauteile und Zuführungen, plant die Bestückreihenfolge und verfolgt die Maschine.",
                "Placement de composants : prépare les travaux, vérifie composants et chargeurs, planifie l'ordre de pose et suit la machine.",
                "Pick and place: prepara os trabalhos, verifica componentes e alimentadores, planeia a ordem de colocação e acompanha a máquina.",
                "贴片：准备作业，检查元件与供料器，规划贴装顺序并跟踪机器。",
                "部品実装：ジョブの準備、部品とフィーダーの確認、実装順の計画、装置の監視を行います。",
            ]),
            Phoenix::Cad => Text8([
                "Mechanical design: sketches, solids and enclosures for the parts of your projects.",
                "Diseño mecánico: croquis, sólidos y cajas para las piezas de sus proyectos.",
                "Progettazione meccanica: schizzi, solidi e contenitori per i pezzi dei vostri progetti.",
                "Mechanische Konstruktion: Skizzen, Volumenkörper und Gehäuse für die Teile Ihrer Projekte.",
                "Conception mécanique : esquisses, solides et boîtiers pour les pièces de vos projets.",
                "Desenho mecânico: esboços, sólidos e caixas para as peças dos seus projetos.",
                "机械设计：为项目零件绘制草图、实体和外壳。",
                "機械設計：プロジェクトの部品に使うスケッチ、ソリッド、ケースを作成します。",
            ]),
            Phoenix::Cnc => Text8([
                "Control of milling and drilling machines: programs, preview, manual movement and checks before cutting.",
                "Control de fresadoras y taladradoras: programas, vista previa, movimiento manual y comprobaciones antes de cortar.",
                "Controllo di fresatrici e foratrici: programmi, anteprima, movimento manuale e controlli prima di tagliare.",
                "Steuerung von Fräs- und Bohrmaschinen: Programme, Vorschau, Handverfahren und Prüfungen vor dem Schnitt.",
                "Pilotage de fraiseuses et de perceuses : programmes, aperçu, déplacement manuel et contrôles avant la coupe.",
                "Controlo de fresadoras e furadoras: programas, pré-visualização, movimento manual e verificações antes do corte.",
                "铣床与钻床控制：程序、预览、手动移动以及切削前检查。",
                "フライス盤とボール盤の制御：プログラム、プレビュー、手動移動、加工前の検査を行います。",
            ]),
            Phoenix::Laser => Text8([
                "Laser cutting and engraving: prepares the jobs with layers, power and speed, and shows them before they run.",
                "Corte y grabado láser: prepara los trabajos con capas, potencia y velocidad, y los muestra antes de ejecutarlos.",
                "Taglio e incisione laser: prepara i lavori con livelli, potenza e velocità e li mostra prima di eseguirli.",
                "Laserschneiden und -gravieren: bereitet Aufträge mit Ebenen, Leistung und Geschwindigkeit vor und zeigt sie vor dem Start.",
                "Découpe et gravure laser : prépare les travaux avec calques, puissance et vitesse, et les montre avant de les lancer.",
                "Corte e gravação a laser: prepara os trabalhos com camadas, potência e velocidade e mostra-os antes de os executar.",
                "激光切割与雕刻：用图层、功率和速度准备作业，并在运行前预览。",
                "レーザー加工と彫刻：レイヤー、出力、速度でジョブを準備し、実行前に表示します。",
            ]),
            Phoenix::Printer3d => Text8([
                "3D printing: checks models, slices them into G-code and works with printers through the EH3D server.",
                "Impresión 3D: comprueba los modelos, los lamina a G-code y trabaja con las impresoras a través del servidor EH3D.",
                "Stampa 3D: controlla i modelli, li converte in G-code e lavora con le stampanti tramite il server EH3D.",
                "3D-Druck: prüft Modelle, zerlegt sie in G-Code und arbeitet über den EH3D-Server mit den Druckern.",
                "Impression 3D : vérifie les modèles, les tranche en G-code et travaille avec les imprimantes via le serveur EH3D.",
                "Impressão 3D: verifica os modelos, fatia-os em G-code e trabalha com as impressoras através do servidor EH3D.",
                "3D 打印：检查模型、切片生成 G-code，并通过 EH3D 服务器使用打印机。",
                "3Dプリント：モデルを検査してG-codeにスライスし、EH3Dサーバーを通してプリンターを扱います。",
            ]),
        }
    }

    /// The notices of data of other people that the program carries (proper names and licences, written in English as their owners write them).
    pub fn notices(self) -> &'static [&'static str] {
        const LASER_DATA: &str =
            "HobbyistLaserCalc, MIT licence, © 2026 hobbyistlasercalc.com (material starting points)";
        const FILAMENT_DATA: &str =
            "Open Filament Database, MIT licence, © 2025 OpenFilamentCollective (filament data)";
        match self {
            Phoenix::Cnc | Phoenix::Laser => &[LASER_DATA],
            Phoenix::Printer3d => &[FILAMENT_DATA],
            Phoenix::Pcb | Phoenix::Pnp | Phoenix::Cad => &[],
        }
    }

    /// What the program says about itself: its name, the version given and the licence.
    pub fn info(self, version: &str) -> AboutInfo {
        AboutInfo {
            app_name: self.name().to_string(),
            version: version.to_string(),
            license: LICENSE.to_string(),
            extra: Vec::new(),
        }
    }
}

/// How a program of the family looks in its window «About»: which one it is and the bytes of its own picture (a PNG that the program puts inside
/// itself with `include_bytes!`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AboutBrand {
    /// The program.
    pub program: Phoenix,
    /// The PNG of its picture.
    pub logo_png: &'static [u8],
}

/// A line of the table of the window: a label, what it says and, when it can be opened, its address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutRow {
    /// The name of the line, in the language of the window.
    pub label: &'static str,
    /// What the line says.
    pub value: String,
    /// The address that opens when it is clicked.
    pub url: Option<String>,
}

/// The lines of the table of the window: version, author, e-mail, the page of the author, the repository and the licence.
pub fn rows(lang: Lang, brand_program: Phoenix, info: &AboutInfo) -> Vec<AboutRow> {
    let row =
        |label: &'static str, value: String, url: Option<String>| AboutRow { label, value, url };
    let repository = brand_program.repository();
    vec![
        row(tr(lang, DockText::Version), info.version.clone(), None),
        row(tr(lang, DockText::Author), AUTHOR.to_string(), None),
        row(
            tr(lang, DockText::Email),
            EMAIL.to_string(),
            Some(format!("mailto:{EMAIL}")),
        ),
        row(
            "GitHub",
            AUTHOR_URL.to_string(),
            Some(AUTHOR_URL.to_string()),
        ),
        row(
            tr(lang, DockText::Repository),
            repository.clone(),
            Some(repository),
        ),
        row(tr(lang, DockText::License), info.license.clone(), None),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_program_has_its_own_name_and_a_colour_that_is_clearly_different() {
        for (i, a) in Phoenix::ALL.iter().enumerate() {
            for b in Phoenix::ALL.iter().skip(i + 1) {
                assert_ne!(a.name(), b.name());
                let distance: u32 = a
                    .accent()
                    .iter()
                    .zip(b.accent())
                    .map(|(x, y)| u32::from(x.abs_diff(y)))
                    .sum();
                assert!(distance >= 75, "{a:?} and {b:?}: {distance}");
            }
        }
    }

    #[test]
    fn the_summary_is_one_sentence_in_eight_languages() {
        for p in Phoenix::ALL {
            let row = p.summary().0;
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty() && text.trim() == *text, "{p:?} {i}");
                assert!(text.ends_with('.') || text.ends_with('。'), "{p:?} {i}");
            }
            assert!(!row[6].is_ascii() && !row[7].is_ascii(), "{p:?}");
            for (i, a) in row.iter().enumerate() {
                assert!(row.iter().skip(i + 1).all(|b| b != a), "{p:?} {i}");
            }
        }
    }

    #[test]
    fn the_lines_say_the_version_given_the_author_and_the_addresses_in_every_language() {
        for p in Phoenix::ALL {
            let info = p.info("9.8.7");
            assert_eq!(info.app_name, p.name());
            assert_eq!(info.license, "GPL-3.0-or-later");
            for lang in Lang::ALL {
                let lines = rows(lang, p, &info);
                let values: Vec<&str> = lines.iter().map(|r| r.value.as_str()).collect();
                let repository = format!("https://github.com/JuanenRac/{}", p.name());
                assert_eq!(
                    values,
                    [
                        "9.8.7",
                        "JuanenRac (Electro Hobby 3D)",
                        "electrohobby3d@gmail.com",
                        "https://github.com/JuanenRac",
                        repository.as_str(),
                        "GPL-3.0-or-later",
                    ]
                );
                assert!(lines.iter().all(|r| !r.label.is_empty()));
                assert_eq!(
                    lines[2].url.as_deref(),
                    Some("mailto:electrohobby3d@gmail.com")
                );
                assert_eq!(lines[4].url.as_deref(), Some(repository.as_str()));
            }
        }
        let spanish = rows(Lang::Es, Phoenix::Cad, &Phoenix::Cad.info("1.0.0"));
        assert_eq!(spanish[1].label, "Autor");
        assert_eq!(spanish[4].label, "Repositorio");
    }

    #[test]
    fn only_the_programs_that_carry_data_of_others_name_them() {
        for p in Phoenix::ALL {
            let expected = matches!(p, Phoenix::Cnc | Phoenix::Laser | Phoenix::Printer3d);
            assert_eq!(!p.notices().is_empty(), expected, "{p:?}");
            assert!(p.notices().iter().all(|n| n.contains("MIT")), "{p:?}");
        }
    }
}
