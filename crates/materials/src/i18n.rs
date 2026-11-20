//! The titles of the error codes in the eight languages of the suite (en, es, de, fr, it, ja, zh
//! and pt, the last one of Portugal).
//!
//! The user interface asks for a title by the key `diag.<code>` as for every code of the suite;
//! a program that has no catalogue of its own can call [`crate::MatCode::title_in`] instead. The
//! texts of the machining family (families, hazards, warnings) are in `machining::texts` and use
//! the same [`Lang`].

use crate::error::MatCode;

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

    /// The index of the language in the tables.
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

const TITLES: [[&str; 8]; 15] = [
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
        "スキーマのバージョンに対応していません",
        "不支持的模式版本",
        "Versão do esquema não suportada",
    ],
    [
        "Duplicate material identifier",
        "Identificador de material duplicado",
        "Doppelte Material-Kennung",
        "Identifiant de matériau en double",
        "Identificatore di materiale duplicato",
        "材料IDが重複しています",
        "材料标识符重复",
        "Identificador de material duplicado",
    ],
    [
        "Identifier does not match the material",
        "El identificador no coincide con el material",
        "Kennung passt nicht zum Material",
        "L'identifiant ne correspond pas au matériau",
        "L'identificatore non corrisponde al materiale",
        "IDが材料と一致しません",
        "标识符与材料不匹配",
        "O identificador não corresponde ao material",
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
        "Source missing or not a file name",
        "Falta la fuente o no es un nombre de fichero",
        "Quelle fehlt oder ist kein Dateiname",
        "Source absente ou qui n'est pas un nom de fichier",
        "Fonte mancante o non è un nome di file",
        "出典がないか、ファイル名ではありません",
        "缺少来源，或来源不是文件名",
        "Fonte em falta ou não é um nome de ficheiro",
    ],
    [
        "Data file not in the embedded list",
        "Fichero de datos fuera de la lista incorporada",
        "Datendatei nicht in der eingebetteten Liste",
        "Fichier de données absent de la liste intégrée",
        "File di dati non presente nell'elenco incorporato",
        "データファイルが組み込みリストにありません",
        "数据文件不在内置列表中",
        "Ficheiro de dados fora da lista incorporada",
    ],
    [
        "Value cannot be interpolated",
        "No se puede interpolar el valor",
        "Wert lässt sich nicht interpolieren",
        "La valeur ne peut pas être interpolée",
        "Il valore non può essere interpolato",
        "値を補間できません",
        "无法对该值进行插值",
        "Não é possível interpolar o valor",
    ],
    [
        "Document record not valid",
        "Registro de documento no válido",
        "Dokumenteintrag ungültig",
        "Fiche de document non valide",
        "Scheda di documento non valida",
        "文書レコードが無効です",
        "文档记录无效",
        "Registo de documento inválido",
    ],
    [
        "Starting value out of range",
        "Valor de partida fuera de rango",
        "Startwert außerhalb des Bereichs",
        "Valeur de départ hors plage",
        "Valore di partenza fuori intervallo",
        "開始値が範囲外です",
        "起始值超出范围",
        "Valor de partida fora do intervalo",
    ],
    [
        "Laser not allowed on this material",
        "Láser no permitido en este material",
        "Laser auf diesem Material nicht erlaubt",
        "Laser interdit sur ce matériau",
        "Laser non consentito su questo materiale",
        "この材料ではレーザーは使用できません",
        "该材料不允许使用激光",
        "Laser não permitido neste material",
    ],
    [
        "No starting points for this process",
        "Sin puntos de partida para este proceso",
        "Keine Startpunkte für diesen Prozess",
        "Aucun point de départ pour ce procédé",
        "Nessun punto di partenza per questo processo",
        "この加工の開始点がありません",
        "此工艺没有起始点",
        "Sem pontos de partida para este processo",
    ],
    [
        "Request outside the known range",
        "Petición fuera del rango conocido",
        "Anfrage außerhalb des bekannten Bereichs",
        "Demande hors de la plage connue",
        "Richiesta fuori dall'intervallo noto",
        "既知の範囲外の要求です",
        "请求超出已知范围",
        "Pedido fora do intervalo conhecido",
    ],
    [
        "Workpiece record not valid",
        "Registro de pieza no válido",
        "Werkstückeintrag ungültig",
        "Fiche de pièce non valide",
        "Scheda del pezzo non valida",
        "加工素材レコードが無効です",
        "工件记录无效",
        "Registo de peça inválido",
    ],
    [
        "Link to the board database broken",
        "Enlace roto con la base de placas",
        "Verknüpfung zur Platinen-Datenbank defekt",
        "Lien rompu avec la base des circuits",
        "Collegamento interrotto con il database dei circuiti",
        "基板データベースへのリンク切れ",
        "与电路板数据库的链接已断开",
        "Ligação à base de dados de placas quebrada",
    ],
];

/// The title of a code in a language.
pub fn code_title(code: MatCode, lang: Lang) -> &'static str {
    TITLES[usize::from(code.number()) - 1][lang.index()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_every_language() {
        for code in MatCode::ALL {
            for lang in Lang::ALL {
                assert!(!code_title(code, lang).is_empty(), "{code} {lang:?}");
            }
            assert_eq!(code_title(code, Lang::En), code.title());
        }
    }

    #[test]
    fn a_title_is_not_the_english_one_in_another_language_unless_it_is_the_same_word() {
        // the Portuguese is the one of Portugal and not a copy of the Spanish
        let same_as_spanish = MatCode::ALL
            .iter()
            .filter(|c| code_title(**c, Lang::Pt) == code_title(**c, Lang::Es))
            .count();
        assert!(same_as_spanish <= 1, "{same_as_spanish}");
        for code in MatCode::ALL {
            for lang in [Lang::Es, Lang::De, Lang::Fr, Lang::It, Lang::Ja, Lang::Zh] {
                assert_ne!(code_title(code, lang), code.title(), "{code} {lang:?}");
            }
        }
    }

    #[test]
    fn the_codes_of_the_languages_read_back() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code("pt-PT"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ZH_cn"), Some(Lang::Zh));
        assert_eq!(Lang::from_code("xx"), None);
    }
}
