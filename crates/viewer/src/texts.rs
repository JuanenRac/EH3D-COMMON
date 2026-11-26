//! The words of the viewer in the eight languages of the suite.

pub use eh3d_view::texts::Lang;

/// The column of a language in the table below: en, es, de, fr, it, ja, zh, pt.
fn column(lang: Lang) -> usize {
    match lang {
        Lang::En => 0,
        Lang::Es => 1,
        Lang::De => 2,
        Lang::Fr => 3,
        Lang::It => 4,
        Lang::Ja => 5,
        Lang::Zh => 6,
        Lang::Pt => 7,
    }
}

macro_rules! texts {
    ($($name:ident: [$($word:literal),* $(,)?];)*) => {
        /// A text of the viewer.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text {
            $($name,)*
        }

        impl Text {
            pub const ALL: &'static [Text] = &[$(Text::$name),*];

            fn words(self) -> [&'static str; 8] {
                match self {
                    $(Text::$name => [$($word),*],)*
                }
            }
        }
    };
}

texts! {
    Empty: ["No attachment is selected.", "No hay ningún adjunto seleccionado.", "Kein Anhang ausgewählt.", "Aucune pièce jointe sélectionnée.", "Nessun allegato selezionato.", "添付ファイルが選択されていません。", "未选择附件。", "Nenhum anexo selecionado."];
    OpenOutside: ["Open outside", "Abrir fuera", "Extern öffnen", "Ouvrir à l’extérieur", "Apri all’esterno", "外部で開く", "在外部打开", "Abrir fora"];
    SaveCopy: ["Save a copy…", "Guardar una copia…", "Kopie speichern…", "Enregistrer une copie…", "Salva una copia…", "コピーを保存…", "另存副本…", "Guardar uma cópia…"];
    ShowPreview: ["Show the preview", "Mostrar la vista previa", "Vorschau anzeigen", "Afficher l’aperçu", "Mostra l’anteprima", "プレビューを表示", "显示预览", "Mostrar a pré-visualização"];
    Confirm: ["I understand, continue", "Entiendo, continuar", "Verstanden, fortfahren", "J’ai compris, continuer", "Ho capito, continua", "理解して続行", "我已了解，继续", "Compreendo, continuar"];
    Cancel: ["Cancel", "Cancelar", "Abbrechen", "Annuler", "Annulla", "キャンセル", "取消", "Cancelar"];
    WarnExecutable: ["This file is a program. It is never run from here.", "Este archivo es un programa. Desde aquí nunca se ejecuta.", "Diese Datei ist ein Programm. Von hier aus wird es nie ausgeführt.", "Ce fichier est un programme. Il n’est jamais exécuté depuis ici.", "Questo file è un programma. Da qui non viene mai eseguito.", "このファイルはプログラムです。ここから実行されることはありません。", "此文件是一个程序。绝不会从这里运行。", "Este ficheiro é um programa. Nunca é executado a partir daqui."];
    WarnScript: ["This file is a script. It is never run from here.", "Este archivo es un script. Desde aquí nunca se ejecuta.", "Diese Datei ist ein Skript. Von hier aus wird es nie ausgeführt.", "Ce fichier est un script. Il n’est jamais exécuté depuis ici.", "Questo file è uno script. Da qui non viene mai eseguito.", "このファイルはスクリプトです。ここから実行されることはありません。", "此文件是一个脚本。绝不会从这里运行。", "Este ficheiro é um script. Nunca é executado a partir daqui."];
    WarnMacros: ["This document has macros, which can run code. Open it only if you trust who sent it.", "Este documento tiene macros, que pueden ejecutar código. Ábrelo solo si confías en quien lo envió.", "Dieses Dokument enthält Makros, die Code ausführen können. Öffnen Sie es nur, wenn Sie dem Absender vertrauen.", "Ce document contient des macros, qui peuvent exécuter du code. Ne l’ouvrez que si vous faites confiance à l’expéditeur.", "Questo documento contiene macro, che possono eseguire codice. Aprilo solo se ti fidi di chi lo ha inviato.", "この文書にはコードを実行できるマクロが含まれています。送信者を信頼できる場合にのみ開いてください。", "此文档包含可执行代码的宏。仅当您信任发送者时才打开。", "Este documento tem macros, que podem executar código. Abra-o apenas se confiar em quem o enviou."];
    WarnArchive: ["This is a compressed archive. Its contents have not been checked.", "Es un archivo comprimido. Su contenido no se ha comprobado.", "Dies ist ein komprimiertes Archiv. Sein Inhalt wurde nicht geprüft.", "Il s’agit d’une archive compressée. Son contenu n’a pas été vérifié.", "È un archivio compresso. Il contenuto non è stato controllato.", "これは圧縮ファイルです。中身は確認されていません。", "这是一个压缩包。其内容未经检查。", "É um arquivo comprimido. O seu conteúdo não foi verificado."];
    WarnActive: ["This page can run code when a browser opens it.", "Esta página puede ejecutar código al abrirla en un navegador.", "Diese Seite kann Code ausführen, wenn ein Browser sie öffnet.", "Cette page peut exécuter du code lorsqu’un navigateur l’ouvre.", "Questa pagina può eseguire codice quando un browser la apre.", "このページはブラウザーで開くとコードを実行する可能性があります。", "用浏览器打开此页面时可能会执行代码。", "Esta página pode executar código quando um navegador a abre."];
    WarnUnknown: ["The program does not know this kind of file.", "El programa no conoce este tipo de archivo.", "Das Programm kennt diesen Dateityp nicht.", "Le programme ne connaît pas ce type de fichier.", "Il programma non conosce questo tipo di file.", "このプログラムはこの種類のファイルを認識できません。", "本程序无法识别此类文件。", "O programa não conhece este tipo de ficheiro."];
    NoPreview: ["There is no preview for this kind of file.", "No hay vista previa para este tipo de archivo.", "Für diesen Dateityp gibt es keine Vorschau.", "Il n’y a pas d’aperçu pour ce type de fichier.", "Non c’è anteprima per questo tipo di file.", "この種類のファイルはプレビューできません。", "此类文件没有预览。", "Não há pré-visualização para este tipo de ficheiro."];
    TextOnly: ["Preview of the text and tables only, without the original formatting.", "Vista previa solo del texto y las tablas, sin el formato original.", "Vorschau nur von Text und Tabellen, ohne die ursprüngliche Formatierung.", "Aperçu du texte et des tableaux seulement, sans la mise en forme d’origine.", "Anteprima solo di testo e tabelle, senza la formattazione originale.", "テキストと表のみのプレビューです。元の書式は反映されません。", "仅预览文本和表格，不含原始格式。", "Pré-visualização apenas do texto e das tabelas, sem a formatação original."];
    Truncated: ["Only the first part is shown.", "Solo se muestra la primera parte.", "Nur der erste Teil wird angezeigt.", "Seule la première partie est affichée.", "È mostrata solo la prima parte.", "最初の部分のみを表示しています。", "仅显示开头部分。", "É mostrada apenas a primeira parte."];
    TooLarge: ["The file is larger than the limit set.", "El archivo supera el límite configurado.", "Die Datei überschreitet die eingestellte Grenze.", "Le fichier dépasse la limite définie.", "Il file supera il limite impostato.", "ファイルが設定された上限を超えています。", "文件超过了设定的上限。", "O ficheiro excede o limite definido."];
    Damaged: ["The file is damaged or is not what its name says.", "El archivo está dañado o no es lo que indica su nombre.", "Die Datei ist beschädigt oder nicht das, was ihr Name sagt.", "Le fichier est endommagé ou n’est pas ce que son nom indique.", "Il file è danneggiato o non è ciò che indica il nome.", "ファイルが壊れているか、名前と中身が一致しません。", "文件已损坏，或内容与其名称不符。", "O ficheiro está danificado ou não é o que o nome indica."];
    EngineMissing: ["The PDF library is not in the program folder. The file can be opened outside.", "Falta la biblioteca de PDF en la carpeta del programa. El archivo se puede abrir fuera.", "Die PDF-Bibliothek fehlt im Programmordner. Die Datei kann extern geöffnet werden.", "La bibliothèque PDF n’est pas dans le dossier du programme. Le fichier peut être ouvert à l’extérieur.", "La libreria PDF non è nella cartella del programma. Il file si può aprire all’esterno.", "プログラムのフォルダーに PDF ライブラリがありません。ファイルは外部で開けます。", "程序文件夹中没有 PDF 库。可以在外部打开此文件。", "A biblioteca de PDF não está na pasta do programa. O ficheiro pode ser aberto fora."];
    EngineRejected: ["The PDF library in the program folder is not the expected one, so it was not loaded.", "La biblioteca de PDF de la carpeta del programa no es la esperada y no se ha cargado.", "Die PDF-Bibliothek im Programmordner ist nicht die erwartete und wurde nicht geladen.", "La bibliothèque PDF du dossier du programme n’est pas celle attendue ; elle n’a pas été chargée.", "La libreria PDF nella cartella del programma non è quella prevista e non è stata caricata.", "プログラムのフォルダーの PDF ライブラリが想定と異なるため、読み込みませんでした。", "程序文件夹中的 PDF 库与预期不符，因此未加载。", "A biblioteca de PDF na pasta do programa não é a esperada e não foi carregada."];
    Password: ["This PDF asks for a password. Open it outside.", "Este PDF pide una contraseña. Ábrelo fuera.", "Dieses PDF verlangt ein Kennwort. Öffnen Sie es extern.", "Ce PDF demande un mot de passe. Ouvrez-le à l’extérieur.", "Questo PDF richiede una password. Aprilo all’esterno.", "この PDF はパスワードが必要です。外部で開いてください。", "此 PDF 需要密码。请在外部打开。", "Este PDF pede uma palavra-passe. Abra-o fora."];
    Refused: ["Programs and scripts are never opened from here. You can save a copy.", "Los programas y los scripts nunca se abren desde aquí. Puedes guardar una copia.", "Programme und Skripte werden von hier nie geöffnet. Sie können eine Kopie speichern.", "Les programmes et les scripts ne sont jamais ouverts depuis ici. Vous pouvez enregistrer une copie.", "Programmi e script non vengono mai aperti da qui. Puoi salvare una copia.", "プログラムとスクリプトはここから開きません。コピーを保存できます。", "程序和脚本绝不会从这里打开。您可以另存副本。", "Os programas e scripts nunca são abertos a partir daqui. Pode guardar uma cópia."];
    Failed: ["The operation could not be done.", "No se pudo realizar la operación.", "Der Vorgang konnte nicht ausgeführt werden.", "L’opération n’a pas pu être effectuée.", "Non è stato possibile eseguire l’operazione.", "操作を実行できませんでした。", "无法完成该操作。", "Não foi possível efetuar a operação."];
    Opening: ["Opening…", "Abriendo…", "Wird geöffnet…", "Ouverture…", "Apertura…", "開いています…", "正在打开…", "A abrir…"];
    Saved: ["The copy was saved.", "Se guardó la copia.", "Die Kopie wurde gespeichert.", "La copie a été enregistrée.", "La copia è stata salvata.", "コピーを保存しました。", "副本已保存。", "A cópia foi guardada."];
    HandedOver: ["A temporary copy was handed to the system to open.", "Se entregó una copia temporal al sistema para abrirla.", "Eine temporäre Kopie wurde dem System zum Öffnen übergeben.", "Une copie temporaire a été confiée au système pour l’ouvrir.", "Una copia temporanea è stata passata al sistema per l’apertura.", "一時コピーをシステムに渡して開きました。", "已将临时副本交给系统打开。", "Foi entregue ao sistema uma cópia temporária para abrir."];
    Page: ["Page", "Página", "Seite", "Page", "Pagina", "ページ", "页", "Página"];
    PreviousPage: ["Previous page", "Página anterior", "Vorherige Seite", "Page précédente", "Pagina precedente", "前のページ", "上一页", "Página anterior"];
    NextPage: ["Next page", "Página siguiente", "Nächste Seite", "Page suivante", "Pagina successiva", "次のページ", "下一页", "Página seguinte"];
    ZoomIn: ["Zoom in", "Acercar", "Vergrößern", "Zoom avant", "Ingrandisci", "拡大", "放大", "Ampliar"];
    ZoomOut: ["Zoom out", "Alejar", "Verkleinern", "Zoom arrière", "Riduci", "縮小", "缩小", "Reduzir"];
    FitWidth: ["Fit width", "Ajustar al ancho", "An Breite anpassen", "Ajuster à la largeur", "Adatta alla larghezza", "幅に合わせる", "适合宽度", "Ajustar à largura"];
    Search: ["Search", "Buscar", "Suchen", "Rechercher", "Cerca", "検索", "搜索", "Procurar"];
    PreviousMatch: ["Previous match", "Coincidencia anterior", "Vorheriger Treffer", "Occurrence précédente", "Risultato precedente", "前の一致", "上一个匹配", "Ocorrência anterior"];
    NextMatch: ["Next match", "Coincidencia siguiente", "Nächster Treffer", "Occurrence suivante", "Risultato successivo", "次の一致", "下一个匹配", "Ocorrência seguinte"];
    NoMatches: ["No matches", "Sin coincidencias", "Keine Treffer", "Aucune occurrence", "Nessun risultato", "一致なし", "无匹配", "Sem ocorrências"];
    Searching: ["Searching…", "Buscando…", "Suche läuft…", "Recherche en cours…", "Ricerca in corso…", "検索中…", "正在搜索…", "A procurar…"];
}

/// The text in a language.
pub fn tr(lang: Lang, text: Text) -> &'static str {
    text.words()[column(lang)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_is_there_in_the_eight_languages() {
        for text in Text::ALL {
            for lang in Lang::ALL {
                assert!(!tr(lang, *text).trim().is_empty(), "{text:?} {lang:?}");
            }
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!tr(lang, *text).is_ascii(), "{text:?} {lang:?}");
            }
            assert_ne!(tr(Lang::De, *text), tr(Lang::En, *text), "{text:?}");
        }
    }
}
