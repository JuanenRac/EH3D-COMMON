//! The texts of the crate in the eight languages of the suite (en, es, de, fr, it, ja, zh and
//! pt, the last one of Portugal): the titles of the error codes and the warnings and notes that
//! come with an exposure.
//!
//! The user interface asks for a title by the key `diag.<code>` as for every code of the suite;
//! a program that has no catalogue of its own can call [`PmCode::title_in`] instead.

use crate::error::PmCode;

/// The languages of the suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    /// English.
    En,
    /// Spanish.
    Es,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese (simplified).
    Zh,
    /// Portuguese of Portugal.
    Pt,
}

impl Lang {
    /// Every language, in the order of the suite.
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two-letter code of the language.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The language that has this code (upper and lower case are the same; a region such as
    /// `pt-PT` or `zh_CN` is cut off), if any.
    pub fn from_code(text: &str) -> Option<Lang> {
        let head = text.trim().split(['-', '_']).next().unwrap_or("");
        Lang::ALL
            .into_iter()
            .find(|l| l.code().eq_ignore_ascii_case(head))
    }

    /// The index of the language in the tables below.
    fn index(self) -> usize {
        self as usize
    }
}

const TITLES: [[&str; 8]; 9] = [
    [
        "Data file not valid",
        "Fichero de datos no válido",
        "Datendatei ungültig",
        "Fichier de données non valide",
        "File di dati non valido",
        "データファイルが無効です",
        "数据文件无效",
        "Ficheiro de dados inválido",
    ],
    [
        "Schema version not supported",
        "Versión de esquema no admitida",
        "Schemaversion nicht unterstützt",
        "Version de schéma non prise en charge",
        "Versione dello schema non supportata",
        "未対応のスキーマバージョンです",
        "不支持的架构版本",
        "Versão do esquema não suportada",
    ],
    [
        "Duplicate identifier",
        "Identificador duplicado",
        "Doppelter Bezeichner",
        "Identifiant en double",
        "Identificatore duplicato",
        "識別子が重複しています",
        "标识符重复",
        "Identificador duplicado",
    ],
    [
        "Identifier does not match the record",
        "El identificador no corresponde al registro",
        "Der Bezeichner passt nicht zum Eintrag",
        "L'identifiant ne correspond pas à l'enregistrement",
        "L'identificatore non corrisponde al record",
        "識別子がレコードと一致しません",
        "标识符与记录不符",
        "O identificador não corresponde ao registo",
    ],
    [
        "Value out of range",
        "Valor fuera de rango",
        "Wert außerhalb des Bereichs",
        "Valeur hors plage",
        "Valore fuori intervallo",
        "値が範囲外です",
        "数值超出范围",
        "Valor fora do intervalo",
    ],
    [
        "Source missing or not valid",
        "Fuente ausente o no válida",
        "Quelle fehlt oder ist ungültig",
        "Source absente ou non valide",
        "Fonte mancante o non valida",
        "出典がないか無効です",
        "来源缺失或无效",
        "Fonte em falta ou inválida",
    ],
    [
        "Data file not in the embedded list",
        "Fichero de datos fuera de la lista incrustada",
        "Datendatei nicht in der eingebetteten Liste",
        "Fichier de données absent de la liste intégrée",
        "File di dati non nell'elenco incorporato",
        "データファイルが組み込み一覧にありません",
        "数据文件不在内嵌列表中",
        "Ficheiro de dados fora da lista incorporada",
    ],
    [
        "Exposure cannot be given",
        "No se puede dar la exposición",
        "Belichtung kann nicht angegeben werden",
        "L'exposition ne peut pas être donnée",
        "L'esposizione non può essere fornita",
        "露光を提示できません",
        "无法给出曝光参数",
        "Não é possível indicar a exposição",
    ],
    [
        "Values contradict each other",
        "Los valores se contradicen",
        "Werte widersprechen sich",
        "Les valeurs se contredisent",
        "I valori si contraddicono",
        "値が互いに矛盾しています",
        "数值相互矛盾",
        "Os valores contradizem-se",
    ],
];

impl PmCode {
    /// The title of the code in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        TITLES[self.number() as usize - 1][lang.index()]
    }
}

/// Why an exposure cannot be given.
#[derive(Debug, Clone, PartialEq)]
pub enum ExposureWarning {
    /// The resin has no exposure settings at all.
    NoPoints,
    /// The resin has no settings for this printer.
    NoPrinter {
        /// The printer asked for.
        printer: String,
    },
    /// The layer height is outside the ones the settings are known for: nothing is extrapolated.
    OutOfRange {
        /// The layer height asked for, in mm.
        layer_mm: f64,
        /// The lowest layer height known, in mm.
        lowest_mm: f64,
        /// The highest layer height known, in mm.
        highest_mm: f64,
    },
}

impl ExposureWarning {
    /// The warning in a language.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            ExposureWarning::NoPoints => [
                "the resin has no exposure settings in the database",
                "la resina no tiene ajustes de exposición en la base de datos",
                "für das Harz gibt es in der Datenbank keine Belichtungswerte",
                "la résine n'a pas de réglages d'exposition dans la base de données",
                "la resina non ha impostazioni di esposizione nel database",
                "この樹脂の露光設定はデータベースにありません",
                "数据库中没有这种树脂的曝光设置",
                "a resina não tem definições de exposição na base de dados",
            ][lang.index()]
                .to_owned(),
            ExposureWarning::NoPrinter { printer } => match lang {
                Lang::En => format!("no exposure settings for the printer {printer}"),
                Lang::Es => format!("no hay ajustes de exposición para la impresora {printer}"),
                Lang::De => format!("keine Belichtungswerte für den Drucker {printer}"),
                Lang::Fr => format!("pas de réglages d'exposition pour l'imprimante {printer}"),
                Lang::It => format!("nessuna impostazione di esposizione per la stampante {printer}"),
                Lang::Ja => format!("プリンター {printer} の露光設定がありません"),
                Lang::Zh => format!("没有打印机 {printer} 的曝光设置"),
                Lang::Pt => format!("não há definições de exposição para a impressora {printer}"),
            },
            ExposureWarning::OutOfRange {
                layer_mm,
                lowest_mm,
                highest_mm,
            } => match lang {
                Lang::En => format!(
                    "layer {layer_mm} mm is outside the {lowest_mm} to {highest_mm} mm that is known: nothing is extrapolated"
                ),
                Lang::Es => format!(
                    "la capa de {layer_mm} mm está fuera de los {lowest_mm} a {highest_mm} mm conocidos: no se extrapola"
                ),
                Lang::De => format!(
                    "die Schicht von {layer_mm} mm liegt außerhalb der bekannten {lowest_mm} bis {highest_mm} mm: es wird nicht extrapoliert"
                ),
                Lang::Fr => format!(
                    "la couche de {layer_mm} mm est hors des {lowest_mm} à {highest_mm} mm connus : pas d'extrapolation"
                ),
                Lang::It => format!(
                    "lo strato di {layer_mm} mm è fuori dai {lowest_mm}-{highest_mm} mm noti: non si estrapola"
                ),
                Lang::Ja => format!(
                    "積層ピッチ {layer_mm} mm は既知の {lowest_mm} ～ {highest_mm} mm の範囲外です。外挿はしません"
                ),
                Lang::Zh => format!(
                    "层高 {layer_mm} mm 超出已知的 {lowest_mm} 至 {highest_mm} mm 范围:不做外推"
                ),
                Lang::Pt => format!(
                    "a camada de {layer_mm} mm está fora dos {lowest_mm} a {highest_mm} mm conhecidos: não se extrapola"
                ),
            },
        }
    }
}

/// What a person should know about an exposure that was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureNote {
    /// The values were interpolated between two known layer heights.
    Interpolated,
    /// The values were not tried on a machine.
    NotVerified,
}

impl ExposureNote {
    /// The note in a language.
    pub fn text(self, lang: Lang) -> &'static str {
        match self {
            ExposureNote::Interpolated => [
                "interpolated between two known layer heights",
                "interpolado entre dos alturas de capa conocidas",
                "zwischen zwei bekannten Schichthöhen interpoliert",
                "interpolé entre deux hauteurs de couche connues",
                "interpolato tra due altezze di strato note",
                "既知の2つの積層ピッチの間を補間した値です",
                "在两个已知层高之间插值得到",
                "interpolado entre duas alturas de camada conhecidas",
            ][lang.index()],
            ExposureNote::NotVerified => [
                "not tried on a machine: test before a long print",
                "no probado en máquina: pruébelo antes de una impresión larga",
                "nicht an einer Maschine erprobt: vor einem langen Druck testen",
                "non essayé sur une machine : testez avant une longue impression",
                "non provato su una macchina: provare prima di una stampa lunga",
                "実機では未確認です。長時間の造形の前にテストしてください",
                "尚未在机器上试过:长时间打印前请先试打",
                "não testado numa máquina: teste antes de uma impressão longa",
            ][lang.index()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_every_language_and_the_english_one_agrees() {
        for code in PmCode::ALL {
            assert_eq!(code.title_in(Lang::En), code.title());
            for lang in Lang::ALL {
                assert!(!code.title_in(lang).is_empty());
            }
            // a title is not the English one by accident in another language
            for lang in &Lang::ALL[1..] {
                assert_ne!(code.title_in(*lang), code.title(), "{code} {}", lang.code());
            }
        }
    }

    #[test]
    fn the_languages_are_found_by_code() {
        assert_eq!(Lang::from_code("pt-PT"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ZH_cn"), Some(Lang::Zh));
        assert_eq!(Lang::from_code("xx"), None);
        for (i, l) in Lang::ALL.iter().enumerate() {
            assert_eq!(l.index(), i);
            assert_eq!(Lang::from_code(l.code()), Some(*l));
        }
    }

    #[test]
    fn warnings_and_notes_exist_in_every_language() {
        let warnings = [
            ExposureWarning::NoPoints,
            ExposureWarning::NoPrinter {
                printer: "p".into(),
            },
            ExposureWarning::OutOfRange {
                layer_mm: 0.2,
                lowest_mm: 0.025,
                highest_mm: 0.1,
            },
        ];
        for w in &warnings {
            let mut seen = std::collections::HashSet::new();
            for lang in Lang::ALL {
                assert!(
                    seen.insert(w.text(lang)),
                    "{w:?} repeats in {}",
                    lang.code()
                );
            }
        }
        for n in [ExposureNote::Interpolated, ExposureNote::NotVerified] {
            let all: std::collections::HashSet<_> = Lang::ALL.iter().map(|l| n.text(*l)).collect();
            assert_eq!(all.len(), 8);
        }
    }
}
