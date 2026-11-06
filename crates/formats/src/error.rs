//! The errors of the readers of the drill file, the plot and the drawing: each one has a stable code (`EH3D-FORMATS-NNN`), the line it is on, the
//! column when the problem is at a place of the line, and its words in the eight languages of the project. The codes do not change from one version to the
//! next; a new problem gets the next number.

use std::fmt;

/// The languages in which the words of a code are written (the eight of the project).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Es,
    It,
    De,
    Fr,
    Pt,
    Zh,
    Ja,
}

impl Language {
    /// The eight languages, in the order of the words of [`Code::message`].
    pub const ALL: [Language; 8] = [
        Language::En,
        Language::Es,
        Language::It,
        Language::De,
        Language::Fr,
        Language::Pt,
        Language::Zh,
        Language::Ja,
    ];

    /// The language of a code of two letters (`en`, `es`, `it`, `de`, `fr`, `pt`, `zh`, `ja`); a variant (`pt-br`, `fr-ca`) is read as its main language.
    pub fn from_code(code: &str) -> Option<Language> {
        let main = code.split(['-', '_']).next().unwrap_or("");
        Language::ALL
            .into_iter()
            .find(|l| l.code().eq_ignore_ascii_case(main))
    }

    /// The code of two letters.
    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Es => "es",
            Language::It => "it",
            Language::De => "de",
            Language::Fr => "fr",
            Language::Pt => "pt",
            Language::Zh => "zh",
            Language::Ja => "ja",
        }
    }

    fn index(self) -> usize {
        Language::ALL
            .iter()
            .position(|l| *l == self)
            .unwrap_or_default()
    }
}

/// The word "line" and the word "column" in each language, for the place of an error.
const PLACE: [(&str, &str); 8] = [
    ("line", "column"),
    ("línea", "columna"),
    ("riga", "colonna"),
    ("Zeile", "Spalte"),
    ("ligne", "colonne"),
    ("linha", "coluna"),
    ("行", "列"),
    ("行", "列"),
];

/// What cannot be read. The number of the code is part of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    /// 001: a line of the drill file that is neither a tool, a coordinate nor a command.
    NcdBadLine,
    /// 002: a coordinate before any tool was chosen.
    NcdHoleBeforeTool,
    /// 003: a drawing whose group code is not a number.
    DxfNotDxf,
    /// 004: a drawing file with nothing in it.
    DxfEmpty,
    /// 005: a drawing without a line, an arc or a circle.
    DxfNoDrawing,
    /// 006: a number of a plot command that cannot be read.
    HpglBadNumber,
    /// 007: a position command of a plot whose last coordinate has no pair.
    HpglOddCoordinates,
}

impl Code {
    /// Every code, in the order of their numbers.
    pub const ALL: [Code; 7] = [
        Code::NcdBadLine,
        Code::NcdHoleBeforeTool,
        Code::DxfNotDxf,
        Code::DxfEmpty,
        Code::DxfNoDrawing,
        Code::HpglBadNumber,
        Code::HpglOddCoordinates,
    ];

    /// The number of the code (1 to 7).
    pub fn number(self) -> u16 {
        match self {
            Code::NcdBadLine => 1,
            Code::NcdHoleBeforeTool => 2,
            Code::DxfNotDxf => 3,
            Code::DxfEmpty => 4,
            Code::DxfNoDrawing => 5,
            Code::HpglBadNumber => 6,
            Code::HpglOddCoordinates => 7,
        }
    }

    /// The stable code, `EH3D-FORMATS-001` and the like.
    pub fn id(self) -> String {
        format!("EH3D-FORMATS-{:03}", self.number())
    }

    /// The words of the code in a language.
    pub fn message(self, lang: Language) -> &'static str {
        let row: [&str; 8] = match self {
            Code::NcdBadLine => [
                "a line of the drill file cannot be read",
                "no se puede leer una línea del fichero de taladros",
                "non si può leggere una riga del file di foratura",
                "eine Zeile der Bohrdatei kann nicht gelesen werden",
                "une ligne du fichier de perçage ne peut pas être lue",
                "não é possível ler uma linha do ficheiro de furação",
                "无法读取钻孔文件中的一行",
                "ドリルファイルの1行を読み取れません",
            ],
            Code::NcdHoleBeforeTool => [
                "a coordinate comes before any tool",
                "hay una coordenada antes de elegir una herramienta",
                "c'è una coordinata prima di qualsiasi utensile",
                "eine Koordinate steht vor jedem Werkzeug",
                "une coordonnée précède tout outil",
                "há uma coordenada antes de qualquer ferramenta",
                "坐标出现在任何刀具之前",
                "工具の指定より前に座標があります",
            ],
            Code::DxfNotDxf => [
                "not a DXF file: the group code is not a number",
                "no es un fichero DXF: el código de grupo no es un número",
                "non è un file DXF: il codice di gruppo non è un numero",
                "keine DXF-Datei: der Gruppencode ist keine Zahl",
                "ce n'est pas un fichier DXF : le code de groupe n'est pas un nombre",
                "não é um ficheiro DXF: o código de grupo não é um número",
                "不是 DXF 文件：组码不是数字",
                "DXF ファイルではありません：グループコードが数値ではありません",
            ],
            Code::DxfEmpty => [
                "the drawing file is empty",
                "el fichero del dibujo está vacío",
                "il file del disegno è vuoto",
                "die Zeichnungsdatei ist leer",
                "le fichier du dessin est vide",
                "o ficheiro do desenho está vazio",
                "图纸文件是空的",
                "図面ファイルが空です",
            ],
            Code::DxfNoDrawing => [
                "the drawing has no lines, arcs or circles",
                "el dibujo no tiene líneas, arcos ni círculos",
                "il disegno non ha linee, archi né cerchi",
                "die Zeichnung enthält keine Linien, Bögen oder Kreise",
                "le dessin n'a ni lignes, ni arcs, ni cercles",
                "o desenho não tem linhas, arcos nem círculos",
                "图纸中没有直线、圆弧或圆",
                "図面に線、円弧、円がありません",
            ],
            Code::HpglBadNumber => [
                "a number of the plot command cannot be read",
                "no se puede leer un número de la orden del trazado",
                "non si può leggere un numero del comando di tracciato",
                "eine Zahl des Plotbefehls kann nicht gelesen werden",
                "un nombre de la commande de tracé ne peut pas être lu",
                "não é possível ler um número do comando do traçado",
                "无法读取绘图命令中的一个数字",
                "プロットコマンドの数値を読み取れません",
            ],
            Code::HpglOddCoordinates => [
                "a position command ends with a coordinate that has no pair",
                "una orden de posición termina con una coordenada sin pareja",
                "un comando di posizione termina con una coordinata senza coppia",
                "ein Positionsbefehl endet mit einer Koordinate ohne Partner",
                "une commande de position se termine par une coordonnée sans paire",
                "um comando de posição termina com uma coordenada sem par",
                "位置命令的最后一个坐标没有配对",
                "位置コマンドの最後の座標に対となる値がありません",
            ],
        };
        row[lang.index()]
    }
}

/// A problem found when reading a file: its code, where it is and the piece of text it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    pub code: Code,
    /// The line, counted from 1; `None` when the problem is about the whole file.
    pub line: Option<usize>,
    /// The column in characters, counted from 1; `None` when the problem is not at a place of the line.
    pub column: Option<usize>,
    /// The text the problem is about (the line, the field); empty when there is none.
    pub detail: String,
}

impl FormatError {
    /// The error with the words of a language: `EH3D-FORMATS-001 línea 3 columna 1: ...`.
    pub fn localized(&self, lang: Language) -> String {
        let (line_word, column_word) = PLACE[lang.index()];
        let mut out = self.code.id();
        if let Some(line) = self.line {
            out.push_str(&format!(" {line_word} {line}"));
            if let Some(column) = self.column {
                out.push_str(&format!(" {column_word} {column}"));
            }
        }
        out.push_str(": ");
        out.push_str(self.code.message(lang));
        if !self.detail.is_empty() {
            out.push_str(&format!(" {:?}", self.detail));
        }
        out
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.localized(Language::En))
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_keep_their_numbers() {
        let ids: Vec<String> = Code::ALL.iter().map(|c| c.id()).collect();
        assert_eq!(
            ids,
            [
                "EH3D-FORMATS-001",
                "EH3D-FORMATS-002",
                "EH3D-FORMATS-003",
                "EH3D-FORMATS-004",
                "EH3D-FORMATS-005",
                "EH3D-FORMATS-006",
                "EH3D-FORMATS-007"
            ]
        );
    }

    #[test]
    fn every_code_is_told_in_the_eight_languages() {
        for code in Code::ALL {
            let english = code.message(Language::En);
            for lang in Language::ALL {
                let words = code.message(lang);
                assert!(!words.trim().is_empty(), "{code:?} in {lang:?}");
                if lang != Language::En {
                    assert_ne!(words, english, "{code:?} in {lang:?}");
                }
            }
        }
        // the Portuguese of Portugal says ficheiro, not the Brazilian arquivo
        assert!(Code::NcdBadLine.message(Language::Pt).contains("ficheiro"));
    }

    #[test]
    fn the_place_and_the_words_are_written_in_the_language_asked() {
        let e = FormatError {
            code: Code::NcdBadLine,
            line: Some(3),
            column: Some(2),
            detail: "T1Cx".into(),
        };
        assert_eq!(
            e.to_string(),
            "EH3D-FORMATS-001 line 3 column 2: a line of the drill file cannot be read \"T1Cx\""
        );
        assert_eq!(
            e.localized(Language::Es),
            "EH3D-FORMATS-001 línea 3 columna 2: no se puede leer una línea del fichero de taladros \"T1Cx\""
        );
        let whole = FormatError {
            code: Code::DxfEmpty,
            line: None,
            column: None,
            detail: String::new(),
        };
        assert_eq!(
            whole.to_string(),
            "EH3D-FORMATS-004: the drawing file is empty"
        );
    }

    #[test]
    fn a_language_is_found_by_its_code_and_a_variant_by_its_main_language() {
        assert_eq!(Language::from_code("de"), Some(Language::De));
        assert_eq!(Language::from_code("pt-br"), Some(Language::Pt));
        assert_eq!(Language::from_code("FR_ca"), Some(Language::Fr));
        assert_eq!(Language::from_code("xx"), None);
    }
}
