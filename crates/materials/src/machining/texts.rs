//! The texts of the machining family in the eight languages of the suite: the names of the
//! families, what each hazard says, and the warnings and notes that come with a starting point.
//! The texts use the impersonal form so that the register is the same in every language.

use super::model::{Family, Hazard};
use super::start::{Quantity, StartNote, StartWarning};
use crate::i18n::Lang;

const FAMILIES: [[&str; 8]; 11] = [
    [
        "Wood", "Madera", "Holz", "Bois", "Legno", "木材", "木材", "Madeira",
    ],
    [
        "Plastic",
        "Plástico",
        "Kunststoff",
        "Plastique",
        "Plastica",
        "プラスチック",
        "塑料",
        "Plástico",
    ],
    [
        "Metal", "Metal", "Metall", "Métal", "Metallo", "金属", "金属", "Metal",
    ],
    [
        "Paper and cardboard",
        "Papel y cartón",
        "Papier und Pappe",
        "Papier et carton",
        "Carta e cartone",
        "紙・段ボール",
        "纸和纸板",
        "Papel e cartão",
    ],
    [
        "Fabric", "Tela", "Stoff", "Tissu", "Tessuto", "布", "织物", "Tecido",
    ],
    [
        "Leather", "Cuero", "Leder", "Cuir", "Cuoio", "革", "皮革", "Couro",
    ],
    [
        "Rubber",
        "Goma",
        "Gummi",
        "Caoutchouc",
        "Gomma",
        "ゴム",
        "橡胶",
        "Borracha",
    ],
    [
        "Foam",
        "Espuma",
        "Schaumstoff",
        "Mousse",
        "Schiuma",
        "フォーム",
        "泡沫材料",
        "Espuma",
    ],
    [
        "Composite",
        "Material compuesto",
        "Verbundwerkstoff",
        "Matériau composite",
        "Materiale composito",
        "複合材料",
        "复合材料",
        "Compósito",
    ],
    [
        "Glass and stone",
        "Vidrio y piedra",
        "Glas und Stein",
        "Verre et pierre",
        "Vetro e pietra",
        "ガラス・石材",
        "玻璃和石材",
        "Vidro e pedra",
    ],
    [
        "Board stock",
        "Material para placas",
        "Leiterplattenmaterial",
        "Matériau pour circuits",
        "Materiale per circuiti",
        "基板素材",
        "电路板板材",
        "Material para placas",
    ],
];

impl Family {
    /// The name of the family in a language.
    pub fn name(self, lang: Lang) -> &'static str {
        FAMILIES[self as usize][lang.index()]
    }
}

const HAZARDS: [[&str; 8]; 12] = [
    [
        "Releases chlorine and hydrochloric acid when burnt: it corrodes the machine and poisons the air.",
        "Libera cloro y ácido clorhídrico al quemarse: corroe la máquina y envenena el aire.",
        "Setzt beim Verbrennen Chlor und Salzsäure frei: Es greift die Maschine an und vergiftet die Luft.",
        "Libère du chlore et de l'acide chlorhydrique en brûlant : il corrode la machine et empoisonne l'air.",
        "Brucia liberando cloro e acido cloridrico: corrode la macchina e avvelena l'aria.",
        "燃焼すると塩素と塩酸を発生し、機械を腐食させ、空気を汚染します。",
        "燃烧时释放氯气和盐酸，会腐蚀机器并污染空气。",
        "Liberta cloro e ácido clorídrico ao queimar: corrói a máquina e envenena o ar.",
    ],
    [
        "Releases hydrogen cyanide and other toxic gases when burnt.",
        "Libera cianuro de hidrógeno y otros gases tóxicos al quemarse.",
        "Setzt beim Verbrennen Blausäure und andere giftige Gase frei.",
        "Libère de l'acide cyanhydrique et d'autres gaz toxiques en brûlant.",
        "Brucia liberando acido cianidrico e altri gas tossici.",
        "燃焼するとシアン化水素などの有毒ガスを発生します。",
        "燃烧时释放氰化氢和其他有毒气体。",
        "Liberta cianeto de hidrogénio e outros gases tóxicos ao queimar.",
    ],
    [
        "Releases formaldehyde when burnt.",
        "Libera formaldehído al quemarse.",
        "Setzt beim Verbrennen Formaldehyd frei.",
        "Libère du formaldéhyde en brûlant.",
        "Brucia liberando formaldeide.",
        "燃焼するとホルムアルデヒドを発生します。",
        "燃烧时释放甲醛。",
        "Liberta formaldeído ao queimar.",
    ],
    [
        "Releases toxic fluorine compounds when burnt.",
        "Libera compuestos de flúor tóxicos al quemarse.",
        "Setzt beim Verbrennen giftige Fluorverbindungen frei.",
        "Libère des composés fluorés toxiques en brûlant.",
        "Brucia liberando composti del fluoro tossici.",
        "燃焼すると有毒なフッ素化合物を発生します。",
        "燃烧时释放有毒的氟化合物。",
        "Liberta compostos de flúor tóxicos ao queimar.",
    ],
    [
        "Contains chromium salts (chrome tanning): the fumes and the ash are toxic.",
        "Contiene sales de cromo (curtido al cromo): los humos y las cenizas son tóxicos.",
        "Enthält Chromsalze (Chromgerbung): Rauch und Asche sind giftig.",
        "Contient des sels de chrome (tannage au chrome) : les fumées et les cendres sont toxiques.",
        "Contiene sali di cromo (concia al cromo): fumi e ceneri sono tossici.",
        "クロム塩（クロムなめし）を含み、煙と灰は有毒です。",
        "含有铬盐（铬鞣），烟雾和灰烬有毒。",
        "Contém sais de crómio (curtimenta ao crómio): os fumos e as cinzas são tóxicos.",
    ],
    [
        "Fibres and fine dust that are harmful to breathe; carbon dust also conducts electricity.",
        "Fibras y polvo fino nocivos al respirarlos; el polvo de carbono además conduce la electricidad.",
        "Fasern und Feinstaub, die beim Einatmen schädlich sind; Kohlenstoffstaub leitet zudem Strom.",
        "Fibres et poussières fines nocives à respirer ; la poussière de carbone conduit aussi l'électricité.",
        "Fibre e polveri sottili nocive se inalate; la polvere di carbonio conduce anche l'elettricità.",
        "吸い込むと有害な繊維と微粉塵が出ます。カーボン粉塵は導電性もあります。",
        "产生吸入有害的纤维和细粉尘；碳粉尘还会导电。",
        "Fibras e pó fino nocivos se inalados; o pó de carbono também conduz eletricidade.",
    ],
    [
        "Gives off toxic fumes when burnt.",
        "Desprende humos tóxicos al quemarse.",
        "Gibt beim Verbrennen giftige Dämpfe ab.",
        "Dégage des fumées toxiques en brûlant.",
        "Sprigiona fumi tossici bruciando.",
        "燃焼すると有毒な煙を出します。",
        "燃烧时产生有毒烟雾。",
        "Liberta fumos tóxicos ao queimar.",
    ],
    [
        "Reflects the beam: dangerous for the eyes and for the optics of the laser.",
        "Refleja el haz: peligroso para los ojos y para la óptica del láser.",
        "Reflektiert den Strahl: gefährlich für die Augen und die Optik des Lasers.",
        "Réfléchit le faisceau : dangereux pour les yeux et pour l'optique du laser.",
        "Riflette il raggio: pericoloso per gli occhi e per l'ottica del laser.",
        "ビームを反射し、目とレーザーの光学系に危険です。",
        "会反射光束，对眼睛和激光光学部件有危险。",
        "Reflete o feixe: perigoso para os olhos e para a ótica do laser.",
    ],
    [
        "Burns easily or keeps burning: never leave the machine alone.",
        "Arde con facilidad o sigue ardiendo: no se debe dejar nunca la máquina sola.",
        "Brennt leicht oder brennt weiter: die Maschine nie unbeaufsichtigt lassen.",
        "Brûle facilement ou continue de brûler : ne jamais laisser la machine sans surveillance.",
        "Brucia facilmente o continua a bruciare: non lasciare mai la macchina incustodita.",
        "燃えやすく、燃え続けます。機械から目を離さないでください。",
        "易燃或持续燃烧：切勿让机器无人看管。",
        "Arde com facilidade ou continua a arder: nunca deixar a máquina sozinha.",
    ],
    [
        "Melts and drips, or chars instead of cutting cleanly.",
        "Se funde y gotea, o se carboniza en vez de cortarse limpio.",
        "Schmilzt und tropft oder verkohlt, statt sauber zu schneiden.",
        "Fond et goutte, ou se carbonise au lieu de se couper proprement.",
        "Fonde e gocciola, oppure carbonizza invece di tagliarsi in modo pulito.",
        "溶けて垂れるか、きれいに切れずに炭化します。",
        "会熔化滴落或碳化，而不是干净地切断。",
        "Derrete e pinga, ou carboniza em vez de cortar limpo.",
    ],
    [
        "Fine dust when cut or milled: extraction and a mask are needed.",
        "Polvo fino al cortarse o fresarse: hacen falta extracción y mascarilla.",
        "Feinstaub beim Schneiden und Fräsen: Absaugung und Maske sind nötig.",
        "Poussière fine à la coupe et au fraisage : aspiration et masque nécessaires.",
        "Polvere sottile nel taglio e nella fresatura: servono aspirazione e mascherina.",
        "切断やフライス加工で微粉塵が出ます。集じんとマスクが必要です。",
        "切割或铣削时产生细粉尘，需要抽尘并佩戴口罩。",
        "Pó fino ao cortar ou fresar: são precisas extração e máscara.",
    ],
    [
        "Does not absorb the beam, so it is not cut or marked (or the beam goes through).",
        "No absorbe el haz, así que no se corta ni se marca (o el haz lo atraviesa).",
        "Absorbiert den Strahl nicht, wird also nicht geschnitten oder markiert (oder der Strahl geht hindurch).",
        "N'absorbe pas le faisceau : il n'est ni coupé ni marqué (ou le faisceau le traverse).",
        "Non assorbe il raggio, quindi non viene tagliato né inciso (o il raggio lo attraversa).",
        "ビームを吸収しないため、切断も刻印もできません（ビームが透過します）。",
        "不吸收光束，因此无法切割或打标（或光束直接穿过）。",
        "Não absorve o feixe, por isso não é cortado nem marcado (ou o feixe atravessa-o).",
    ],
];

impl Hazard {
    /// What the hazard says, in a language.
    pub fn text(self, lang: Lang) -> &'static str {
        HAZARDS[self as usize][lang.index()]
    }
}

const QUANTITIES: [[&str; 8]; 3] = [
    [
        "Cutter diameter",
        "Diámetro de la fresa",
        "Fräserdurchmesser",
        "Diamètre de la fraise",
        "Diametro della fresa",
        "工具径",
        "刀具直径",
        "Diâmetro da fresa",
    ],
    [
        "Thickness",
        "Espesor",
        "Dicke",
        "Épaisseur",
        "Spessore",
        "厚さ",
        "厚度",
        "Espessura",
    ],
    [
        "Laser power",
        "Potencia del láser",
        "Laserleistung",
        "Puissance du laser",
        "Potenza del laser",
        "レーザー出力",
        "激光功率",
        "Potência do laser",
    ],
];

impl Quantity {
    /// The name of the quantity in a language.
    pub fn name(self, lang: Lang) -> &'static str {
        QUANTITIES[self as usize][lang.index()]
    }

    /// The unit that goes after a value of the quantity.
    pub fn unit(self) -> &'static str {
        match self {
            Quantity::Diameter | Quantity::Thickness => "mm",
            Quantity::Power => "W",
        }
    }
}

const NO_POINTS: [&str; 8] = [
    "This material has no starting points.",
    "Este material no tiene puntos de partida.",
    "Für dieses Material gibt es keine Startpunkte.",
    "Ce matériau n'a pas de points de départ.",
    "Questo materiale non ha punti di partenza.",
    "この材料には開始点がありません。",
    "该材料没有起始点。",
    "Este material não tem pontos de partida.",
];
const NO_PROCESS: [&str; 8] = [
    "There are no starting points for this process.",
    "No hay puntos de partida para este proceso.",
    "Für diesen Prozess gibt es keine Startpunkte.",
    "Il n'y a pas de points de départ pour ce procédé.",
    "Non ci sono punti di partenza per questo processo.",
    "この加工の開始点はありません。",
    "此工艺没有起始点。",
    "Não há pontos de partida para este processo.",
];
const FORBIDDEN: [&str; 8] = [
    "A laser must not be used on this material.",
    "No se debe usar un láser con este material.",
    "Auf diesem Material darf kein Laser verwendet werden.",
    "Il ne faut pas utiliser de laser sur ce matériau.",
    "Su questo materiale non si deve usare il laser.",
    "この材料にレーザーを使ってはいけません。",
    "不得在该材料上使用激光。",
    "Não se deve usar laser neste material.",
];
/// The sentence of a request outside the known range: `{q}`, `{r}`, `{lo}`, `{hi}` and `{u}`.
const OUT_OF_RANGE: [&str; 8] = [
    "{q}: {r} {u} is outside the known range {lo} to {hi} {u}; nothing is extrapolated.",
    "{q}: {r} {u} queda fuera del rango conocido, de {lo} a {hi} {u}; no se extrapola.",
    "{q}: {r} {u} liegt außerhalb des bekannten Bereichs von {lo} bis {hi} {u}; es wird nicht extrapoliert.",
    "{q} : {r} {u} est hors de la plage connue, de {lo} à {hi} {u} ; rien n'est extrapolé.",
    "{q}: {r} {u} è fuori dall'intervallo noto, da {lo} a {hi} {u}; non si estrapola.",
    "{q}：{r} {u} は既知の範囲（{lo}～{hi} {u}）の外です。外挿はしません。",
    "{q}：{r} {u} 超出已知范围（{lo} 至 {hi} {u}），不做外推。",
    "{q}: {r} {u} está fora do intervalo conhecido, de {lo} a {hi} {u}; não se extrapola.",
];

fn number(v: f64) -> String {
    let text = format!("{v:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl StartWarning {
    /// The warning in a language.
    pub fn text(&self, lang: Lang) -> String {
        let i = lang.index();
        match self {
            StartWarning::NoPoints => NO_POINTS[i].to_owned(),
            StartWarning::NoSuchProcess => NO_PROCESS[i].to_owned(),
            StartWarning::LaserForbidden => FORBIDDEN[i].to_owned(),
            StartWarning::OutOfRange {
                quantity,
                requested,
                lowest,
                highest,
            } => OUT_OF_RANGE[i]
                .replace("{q}", quantity.name(lang))
                .replace("{r}", &number(*requested))
                .replace("{lo}", &number(*lowest))
                .replace("{hi}", &number(*highest))
                .replace("{u}", quantity.unit()),
        }
    }
}

const NOTE_INTERPOLATED: [&str; 8] = [
    "Between two known points: check on a scrap piece.",
    "Entre dos puntos conocidos: comprobar en una pieza de desecho.",
    "Zwischen zwei bekannten Punkten: an einem Reststück prüfen.",
    "Entre deux points connus : à vérifier sur une chute.",
    "Tra due punti noti: verificare su uno scarto.",
    "既知の2点の間の値です。端材で確認してください。",
    "介于两个已知点之间，请先在废料上试验。",
    "Entre dois pontos conhecidos: verificar numa sobra.",
];
const NOTE_NOT_VERIFIED: [&str; 8] = [
    "Starting point, not tried on a machine: begin with less power or feed and raise it.",
    "Punto de partida, no probado en una máquina: empezar con menos potencia o avance y subirlo.",
    "Startwert, nicht an einer Maschine erprobt: mit weniger Leistung oder Vorschub beginnen und steigern.",
    "Point de départ non essayé sur une machine : commencer avec moins de puissance ou d'avance, puis augmenter.",
    "Punto di partenza non provato su una macchina: iniziare con meno potenza o avanzamento e aumentare.",
    "開始点であり、実機では未確認です。出力や送りを下げて始め、徐々に上げてください。",
    "仅为起始点，未在机器上试过：先用较低的功率或进给，再逐步提高。",
    "Ponto de partida, não testado numa máquina: começar com menos potência ou avanço e aumentar.",
];

impl StartNote {
    /// The note in a language.
    pub fn text(self, lang: Lang) -> &'static str {
        match self {
            StartNote::Interpolated => NOTE_INTERPOLATED[lang.index()],
            StartNote::NotVerified => NOTE_NOT_VERIFIED[lang.index()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_every_language_and_the_others_are_not_the_english() {
        for lang in Lang::ALL {
            for f in Family::ALL {
                assert!(!f.name(lang).is_empty());
            }
            for h in Hazard::ALL {
                assert!(!h.text(lang).is_empty());
                if !matches!(lang, Lang::En) {
                    assert_ne!(h.text(lang), h.text(Lang::En), "{h:?} {lang:?}");
                }
            }
            for n in [StartNote::Interpolated, StartNote::NotVerified] {
                assert!(!n.text(lang).is_empty());
            }
        }
    }

    #[test]
    fn the_sentence_of_a_request_outside_the_range_fills_every_place() {
        let w = StartWarning::OutOfRange {
            quantity: Quantity::Thickness,
            requested: 12.0,
            lowest: 3.0,
            highest: 6.5,
        };
        for lang in Lang::ALL {
            let t = w.text(lang);
            assert!(!t.contains('{') && !t.contains('}'), "{t}");
            assert!(
                t.contains("12") && t.contains("3") && t.contains("6.5"),
                "{t}"
            );
            assert!(t.contains("mm"), "{t}");
        }
    }

    #[test]
    fn the_portuguese_is_the_one_of_portugal() {
        assert!(Hazard::Chromium.text(Lang::Pt).contains("crómio"));
        assert!(Hazard::Fluorine.text(Lang::Pt).contains("Liberta"));
        assert_eq!(Family::Wood.name(Lang::Pt), "Madeira");
    }
}
