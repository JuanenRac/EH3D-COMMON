//! The words of the frame (the menus, the dialogs, the warnings) in the eight languages of the project, and the type a program uses for the words
//! of its own panels and menus.
//!
//! Each row has the eight texts in the order English, Spanish, Italian, German, French, Portuguese (of Portugal), Chinese, Japanese; the type of the
//! table makes the compiler refuse a row with fewer or more. A language beyond the first eight speaks its main language when it is a variant
//! (`pt-br`, `fr-ca`) and English otherwise.

use eh3d_view::texts::{Lang, CORE_LANGS};

/// Which of the eight columns a language reads: its own, its main language when it is a variant, or English.
pub fn lang_index(lang: Lang) -> usize {
    let pos = |l: Lang| Lang::ALL.iter().position(|x| *x == l).unwrap_or(0);
    let i = pos(lang);
    if i < CORE_LANGS {
        return i;
    }
    match lang.fallback() {
        Some(main) if pos(main) < CORE_LANGS => pos(main),
        _ => 0,
    }
}

/// A text a program gives in eight languages (the title of a panel, the name of a menu, an entry of it), in the order English, Spanish, Italian,
/// German, French, Portuguese (of Portugal), Chinese, Japanese.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Text8(pub [&'static str; 8]);

impl Text8 {
    /// The text in a language.
    pub fn get(&self, lang: Lang) -> &'static str {
        self.0[lang_index(lang)]
    }

    /// The same text in every language (a proper name or a number).
    pub const fn same(t: &'static str) -> Text8 {
        Text8([t, t, t, t, t, t, t, t])
    }
}

/// Puts the values in the `{name}` marks of a text.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in values {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

macro_rules! dock_texts {
    ($($name:ident => [$($t:expr),+ $(,)?]),+ $(,)?) => {
        /// A word or sentence of the frame.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum DockText { $($name),+ }
        /// Every text, in the order of the table.
        pub const ALL_DOCK_TEXTS: &[DockText] = &[$(DockText::$name),+];
        const TABLE: &[[&str; 8]] = &[$([$($t),+]),+];
        impl DockText {
            fn index(self) -> usize {
                ALL_DOCK_TEXTS.iter().position(|t| *t == self).unwrap_or(0)
            }
            /// The name of the text, as a file of words would write it.
            pub fn name(self) -> &'static str {
                match self { $(DockText::$name => stringify!($name)),+ }
            }
        }
    };
}

dock_texts! {
    MenuFile => ["File", "Archivo", "File", "Datei", "Fichier", "Ficheiro", "文件", "ファイル"],
    MenuEdit => ["Edit", "Editar", "Modifica", "Bearbeiten", "Édition", "Editar", "编辑", "編集"],
    MenuView => ["View", "Ver", "Visualizza", "Ansicht", "Affichage", "Ver", "视图", "表示"],
    MenuTools => ["Tools", "Herramientas", "Strumenti", "Werkzeuge", "Outils", "Ferramentas", "工具", "ツール"],
    MenuWindow => ["Window", "Ventana", "Finestra", "Fenster", "Fenêtre", "Janela", "窗口", "ウィンドウ"],
    MenuHelp => ["Help", "Ayuda", "Aiuto", "Hilfe", "Aide", "Ajuda", "帮助", "ヘルプ"],
    New => ["New", "Nuevo", "Nuovo", "Neu", "Nouveau", "Novo", "新建", "新規"],
    Open => ["Open…", "Abrir…", "Apri…", "Öffnen…", "Ouvrir…", "Abrir…", "打开…", "開く…"],
    OpenRecent => ["Open recent", "Abrir reciente", "Apri recenti", "Zuletzt geöffnet", "Ouvrir récent", "Abrir recentes", "最近打开", "最近使ったファイル"],
    NoRecent => ["(none)", "(ninguno)", "(nessuno)", "(keine)", "(aucun)", "(nenhum)", "（无）", "（なし）"],
    ClearRecent => ["Clear the list", "Vaciar la lista", "Svuota l'elenco", "Liste leeren", "Vider la liste", "Limpar a lista", "清空列表", "一覧を消去"],
    Save => ["Save", "Guardar", "Salva", "Speichern", "Enregistrer", "Guardar", "保存", "保存"],
    SaveAs => ["Save as…", "Guardar como…", "Salva con nome…", "Speichern unter…", "Enregistrer sous…", "Guardar como…", "另存为…", "名前を付けて保存…"],
    Export => ["Export…", "Exportar…", "Esporta…", "Exportieren…", "Exporter…", "Exportar…", "导出…", "エクスポート…"],
    Quit => ["Quit", "Salir", "Esci", "Beenden", "Quitter", "Sair", "退出", "終了"],
    Undo => ["Undo", "Deshacer", "Annulla", "Rückgängig", "Annuler", "Anular", "撤销", "元に戻す"],
    Redo => ["Redo", "Rehacer", "Ripeti", "Wiederholen", "Rétablir", "Refazer", "重做", "やり直し"],
    Cut => ["Cut", "Cortar", "Taglia", "Ausschneiden", "Couper", "Cortar", "剪切", "切り取り"],
    Copy => ["Copy", "Copiar", "Copia", "Kopieren", "Copier", "Copiar", "复制", "コピー"],
    Paste => ["Paste", "Pegar", "Incolla", "Einfügen", "Coller", "Colar", "粘贴", "貼り付け"],
    Preferences => ["Preferences…", "Preferencias…", "Preferenze…", "Einstellungen…", "Préférences…", "Preferências…", "首选项…", "環境設定…"],
    Panels => ["Panels", "Paneles", "Pannelli", "Bereiche", "Panneaux", "Painéis", "面板", "パネル"],
    RestoreLayout => ["Restore the layout", "Restaurar la disposición", "Ripristina la disposizione", "Anordnung zurücksetzen", "Restaurer la disposition", "Repor a disposição", "恢复布局", "レイアウトを初期化"],
    LockLayout => ["Lock the layout", "Bloquear la disposición", "Blocca la disposizione", "Anordnung sperren", "Verrouiller la disposition", "Bloquear a disposição", "锁定布局", "レイアウトを固定"],
    Workspaces => ["Workspaces", "Espacios de trabajo", "Aree di lavoro", "Arbeitsbereiche", "Espaces de travail", "Áreas de trabalho", "工作区", "ワークスペース"],
    SaveWorkspace => ["Save the layout as…", "Guardar la disposición como…", "Salva la disposizione come…", "Anordnung speichern als…", "Enregistrer la disposition sous…", "Guardar a disposição como…", "将布局另存为…", "レイアウトを名前を付けて保存…"],
    DeleteWorkspace => ["Delete this workspace", "Borrar este espacio", "Elimina quest'area", "Diesen Arbeitsbereich löschen", "Supprimer cet espace", "Apagar esta área", "删除此工作区", "このワークスペースを削除"],
    WorkspaceName => ["Name of the workspace", "Nombre del espacio de trabajo", "Nome dell'area di lavoro", "Name des Arbeitsbereichs", "Nom de l'espace de travail", "Nome da área de trabalho", "工作区名称", "ワークスペース名"],
    Accept => ["Accept", "Aceptar", "Accetta", "Übernehmen", "Valider", "Aceitar", "确定", "決定"],
    Cancel => ["Cancel", "Cancelar", "Annulla", "Abbrechen", "Annuler", "Cancelar", "取消", "キャンセル"],
    Close => ["Close", "Cerrar", "Chiudi", "Schließen", "Fermer", "Fechar", "关闭", "閉じる"],
    Float => ["Float", "Flotar", "Rendi mobile", "Schweben lassen", "Détacher", "Flutuar", "浮动", "フロート"],
    Dock => ["Dock", "Acoplar", "Aggancia", "Andocken", "Ancrer", "Acoplar", "停靠", "ドッキング"],
    Hide => ["Hide", "Ocultar", "Nascondi", "Ausblenden", "Masquer", "Ocultar", "隐藏", "非表示"],
    AutoHide => ["Auto-hide", "Ocultar automáticamente", "Nascondi automaticamente", "Automatisch ausblenden", "Masquer automatiquement", "Ocultar automaticamente", "自动隐藏", "自動的に隠す"],
    MoveTo => ["Move to", "Mover a", "Sposta in", "Verschieben nach", "Déplacer vers", "Mover para", "移动到", "移動先"],
    Left => ["Left", "Izquierda", "Sinistra", "Links", "Gauche", "Esquerda", "左", "左"],
    Right => ["Right", "Derecha", "Destra", "Rechts", "Droite", "Direita", "右", "右"],
    Top => ["Top", "Arriba", "Alto", "Oben", "Haut", "Cima", "上", "上"],
    Bottom => ["Bottom", "Abajo", "Basso", "Unten", "Bas", "Baixo", "下", "下"],
    DropAsTab => ["As a tab here", "Como pestaña aquí", "Come scheda qui", "Als Registerkarte hier", "En onglet ici", "Como separador aqui", "作为标签页放在此处", "ここにタブとして追加"],
    Maximize => ["Maximize", "Maximizar", "Massimizza", "Maximieren", "Agrandir", "Maximizar", "最大化", "最大化"],
    RestoreSize => ["Restore size", "Restaurar tamaño", "Ripristina dimensione", "Größe wiederherstellen", "Restaurer la taille", "Repor o tamanho", "还原大小", "元のサイズに戻す"],
    NextTab => ["Next tab", "Pestaña siguiente", "Scheda successiva", "Nächster Reiter", "Onglet suivant", "Separador seguinte", "下一个标签页", "次のタブ"],
    PrevTab => ["Previous tab", "Pestaña anterior", "Scheda precedente", "Vorheriger Reiter", "Onglet précédent", "Separador anterior", "上一个标签页", "前のタブ"],
    About => ["About…", "Acerca de…", "Informazioni…", "Über…", "À propos…", "Sobre…", "关于…", "バージョン情報…"],
    Shortcuts => ["Keyboard shortcuts", "Atajos de teclado", "Scorciatoie da tastiera", "Tastenkürzel", "Raccourcis clavier", "Atalhos de teclado", "键盘快捷键", "キーボードショートカット"],
    OnlineHelp => ["Online help", "Ayuda en línea", "Guida online", "Online-Hilfe", "Aide en ligne", "Ajuda online", "在线帮助", "オンラインヘルプ"],
    CheckUpdates => ["Check for updates…", "Buscar actualizaciones…", "Verifica aggiornamenti…", "Nach Updates suchen…", "Rechercher des mises à jour…", "Procurar atualizações…", "检查更新…", "アップデートを確認…"],
    Version => ["Version", "Versión", "Versione", "Version", "Version", "Versão", "版本", "バージョン"],
    License => ["License", "Licencia", "Licenza", "Lizenz", "Licence", "Licença", "许可证", "ライセンス"],
    Action => ["Action", "Acción", "Azione", "Aktion", "Action", "Ação", "操作", "操作"],
    Shortcut => ["Shortcut", "Atajo", "Scorciatoia", "Kürzel", "Raccourci", "Atalho", "快捷键", "ショートカット"],
    Trademarks => [
        "Other product and company names are trademarks of their owners and appear here only to name the formats and tools the program works with.",
        "Los demás nombres de productos y empresas son marcas de sus propietarios y aparecen aquí solo para nombrar los formatos y herramientas con los que trabaja el programa.",
        "Gli altri nomi di prodotti e aziende sono marchi dei rispettivi proprietari e compaiono qui solo per indicare i formati e gli strumenti con cui il programma lavora.",
        "Weitere Produkt- und Firmennamen sind Marken ihrer jeweiligen Inhaber und erscheinen hier nur, um die Formate und Werkzeuge zu benennen, mit denen das Programm arbeitet.",
        "Les autres noms de produits et d'entreprises sont des marques de leurs propriétaires et ne figurent ici que pour nommer les formats et les outils avec lesquels le programme travaille.",
        "Os restantes nomes de produtos e de empresas são marcas dos respetivos proprietários e surgem aqui apenas para nomear os formatos e as ferramentas com que o programa trabalha.",
        "其他产品和公司名称为其各自所有者的商标，在此仅用于说明本程序所兼容的格式和工具。",
        "その他の製品名および会社名は各所有者の商標であり、ここではプログラムが扱う形式やツールを示すためにのみ記載しています。",
    ],
    LayoutLocked => ["The layout is locked", "La disposición está bloqueada", "La disposizione è bloccata", "Die Anordnung ist gesperrt", "La disposition est verrouillée", "A disposição está bloqueada", "布局已锁定", "レイアウトは固定されています"],
    WarnCorrupt => ["The saved layout is damaged; the factory layout is used", "La disposición guardada está dañada; se usa la de fábrica", "La disposizione salvata è danneggiata; si usa quella di fabbrica", "Die gespeicherte Anordnung ist beschädigt; die Werksanordnung wird verwendet", "La disposition enregistrée est endommagée ; celle d'usine est utilisée", "A disposição guardada está danificada; é usada a de fábrica", "已保存的布局已损坏，改用出厂布局", "保存されたレイアウトが壊れているため、初期レイアウトを使います"],
    WarnVersion => ["The saved layout is from another version; the factory layout is used", "La disposición guardada es de otra versión; se usa la de fábrica", "La disposizione salvata è di un'altra versione; si usa quella di fabbrica", "Die gespeicherte Anordnung stammt aus einer anderen Version; die Werksanordnung wird verwendet", "La disposition enregistrée vient d'une autre version ; celle d'usine est utilisée", "A disposição guardada é de outra versão; é usada a de fábrica", "已保存的布局来自其他版本，改用出厂布局", "保存されたレイアウトは別のバージョンのものなので、初期レイアウトを使います"],
    WarnRepaired => ["The saved layout had problems and was repaired", "La disposición guardada tenía problemas y se ha reparado", "La disposizione salvata aveva problemi ed è stata riparata", "Die gespeicherte Anordnung hatte Fehler und wurde repariert", "La disposition enregistrée avait des problèmes et a été réparée", "A disposição guardada tinha problemas e foi reparada", "已保存的布局存在问题，已修复", "保存されたレイアウトに問題があったため修復しました"],
    WarnKeys => ["A shortcut in the settings could not be read and was ignored", "Un atajo de los ajustes no se pudo leer y se ha ignorado", "Una scorciatoia nelle impostazioni non è leggibile ed è stata ignorata", "Ein Tastenkürzel in den Einstellungen war nicht lesbar und wurde ignoriert", "Un raccourci des réglages est illisible et a été ignoré", "Um atalho das definições não pôde ser lido e foi ignorado", "设置中有一个快捷键无法读取，已忽略", "設定のショートカットが読み取れなかったため無視しました"],
    ShortcutConflict => ["Two actions use the same shortcut", "Dos acciones usan el mismo atajo", "Due azioni usano la stessa scorciatoia", "Zwei Aktionen verwenden dasselbe Tastenkürzel", "Deux actions utilisent le même raccourci", "Duas ações usam o mesmo atalho", "两个操作使用了相同的快捷键", "2つの操作が同じショートカットを使っています"],
    StateDocked => ["docked", "acoplado", "agganciato", "angedockt", "ancré", "acoplado", "已停靠", "ドッキング中"],
    StateFloating => ["floating", "flotante", "mobile", "schwebend", "détaché", "flutuante", "浮动", "フロート中"],
    StateHidden => ["hidden", "oculto", "nascosto", "ausgeblendet", "masqué", "oculto", "已隐藏", "非表示"],
    StateAutoHidden => ["auto-hidden", "ocultado automáticamente", "nascosto automaticamente", "automatisch ausgeblendet", "masqué automatiquement", "ocultado automaticamente", "已自动隐藏", "自動的に隠れています"],
    PanelLabel => ["Panel {name}, {state}", "Panel {name}, {state}", "Pannello {name}, {state}", "Bereich {name}, {state}", "Panneau {name}, {state}", "Painel {name}, {state}", "面板 {name}，{state}", "パネル {name}（{state}）"],
    DropHere => ["Drop here", "Suelta aquí", "Rilascia qui", "Hier ablegen", "Déposer ici", "Largar aqui", "在此放下", "ここにドロップ"],
    Recent => ["Recent files", "Archivos recientes", "File recenti", "Zuletzt verwendete Dateien", "Fichiers récents", "Ficheiros recentes", "最近的文件", "最近のファイル"],
}

/// The text in a language.
pub fn tr(lang: Lang, text: DockText) -> &'static str {
    TABLE[text.index()][lang_index(lang)]
}

/// The text of the label of a panel for a screen reader: «Panel Layers, docked».
pub fn panel_label(lang: Lang, name: &str, state: DockText) -> String {
    fill(
        tr(lang, DockText::PanelLabel),
        &[("name", name), ("state", tr(lang, state))],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_has_eight_filled_columns() {
        for t in ALL_DOCK_TEXTS {
            for (i, s) in TABLE[t.index()].iter().enumerate() {
                assert!(!s.trim().is_empty(), "{} column {i}", t.name());
                assert_eq!(s.trim(), *s, "{} column {i} has spaces at an end", t.name());
            }
        }
        assert!(ALL_DOCK_TEXTS.len() >= 60);
    }

    #[test]
    fn chinese_and_japanese_are_written_in_their_scripts_and_the_others_are_translated() {
        for t in ALL_DOCK_TEXTS {
            let row = &TABLE[t.index()];
            assert!(!row[6].is_ascii(), "{} zh", t.name());
            assert!(!row[7].is_ascii(), "{} ja", t.name());
            // Spanish, Italian, German, French and Portuguese never carry Chinese or Japanese letters
            for (i, text) in row.iter().enumerate().take(6).skip(1) {
                assert!(
                    !text.chars().any(|c| ('\u{3040}'..='\u{9fff}').contains(&c)),
                    "{} column {i}",
                    t.name()
                );
            }
        }
    }

    #[test]
    fn the_formal_register_and_the_portuguese_of_portugal() {
        for t in ALL_DOCK_TEXTS {
            let row = &TABLE[t.index()];
            for w in [" tu ", " toi ", " ton ", " ta "] {
                assert!(
                    !format!(" {} ", row[4]).to_lowercase().contains(w),
                    "{} fr",
                    t.name()
                );
            }
            for w in [" du ", " dir ", " dein "] {
                assert!(
                    !format!(" {} ", row[3]).to_lowercase().contains(w),
                    "{} de",
                    t.name()
                );
            }
            for w in [" tu ", " teu ", " tua "] {
                assert!(
                    !format!(" {} ", row[5]).to_lowercase().contains(w),
                    "{} pt",
                    t.name()
                );
            }
            // the words of Brazil
            for w in ["arquivo", "tela", "celular", "usuário", "salvar"] {
                assert!(!row[5].to_lowercase().contains(w), "{} pt-PT {w}", t.name());
            }
        }
    }

    #[test]
    fn languages_beyond_the_first_eight_read_a_main_language() {
        assert_eq!(tr(Lang::PtBr, DockText::MenuFile), "Ficheiro");
        assert_eq!(tr(Lang::FrCa, DockText::MenuFile), "Fichier");
        assert_eq!(tr(Lang::Pl, DockText::MenuFile), "File");
        assert_eq!(tr(Lang::Ja, DockText::MenuHelp), "ヘルプ");
    }

    #[test]
    fn marks_are_filled() {
        let s = panel_label(Lang::Es, "Capas", DockText::StateDocked);
        assert_eq!(s, "Panel Capas, acoplado");
        assert!(!panel_label(Lang::Zh, "图层", DockText::StateHidden).contains('{'));
        for lang in Lang::ALL {
            assert!(!panel_label(lang, "x", DockText::StateFloating).contains('{'));
        }
        assert_eq!(fill("{a}-{b}", &[("a", "1"), ("b", "2")]), "1-2");
    }

    #[test]
    fn text8_reads_by_language() {
        let t = Text8([
            "Layers",
            "Capas",
            "Livelli",
            "Ebenen",
            "Calques",
            "Camadas",
            "图层",
            "レイヤー",
        ]);
        assert_eq!(t.get(Lang::En), "Layers");
        assert_eq!(t.get(Lang::Pt), "Camadas");
        assert_eq!(t.get(Lang::PtBr), "Camadas");
        assert_eq!(t.get(Lang::Ru), "Layers");
        assert_eq!(Text8::same("3D").get(Lang::Ja), "3D");
    }
}
