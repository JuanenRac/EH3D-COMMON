//! The words of the attachment panel in the eight languages of the suite.

pub use eh3d_project::suitetext::fill;
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
        /// A text of the panel; `{0}` and `{1}` are filled by [`fill`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text {
            $(
                #[doc = stringify!($name)]
                $name,
            )*
        }

        impl Text {
            /// Every text.
            pub const ALL: &'static [Text] = &[$(Text::$name),*];

            /// The text in the eight languages, in the order en, es, de, fr, it, ja, zh, pt.
            pub fn words(self) -> [&'static str; 8] {
                match self {
                    $(Text::$name => [$($word),*],)*
                }
            }
        }
    };
}

texts! {
    Title: ["Attachments", "Adjuntos", "Anhänge", "Pièces jointes", "Allegati", "添付ファイル", "附件", "Anexos"];
    OpenProject: ["Open suite project…", "Abrir proyecto de la suite…", "Suite-Projekt öffnen…", "Ouvrir un projet de la suite…", "Apri progetto della suite…", "スイートプロジェクトを開く…", "打开套件项目…", "Abrir projeto da suite…"];
    CloseProject: ["Close the project", "Cerrar el proyecto", "Projekt schließen", "Fermer le projet", "Chiudi il progetto", "プロジェクトを閉じる", "关闭项目", "Fechar o projeto"];
    ProjectName: ["Project: {0}", "Proyecto: {0}", "Projekt: {0}", "Projet : {0}", "Progetto: {0}", "プロジェクト：{0}", "项目：{0}", "Projeto: {0}"];
    NoProject: ["Open or save a suite project (.eh3d) to keep data sheets and manuals with it.", "Abre o guarda un proyecto de la suite (.eh3d) para guardar con él hojas de datos y manuales.", "Öffnen oder speichern Sie ein Suite-Projekt (.eh3d), um Datenblätter und Handbücher darin aufzubewahren.", "Ouvrez ou enregistrez un projet de la suite (.eh3d) pour y conserver fiches techniques et manuels.", "Apri o salva un progetto della suite (.eh3d) per conservarvi schede tecniche e manuali.", "データシートやマニュアルを保存するには、スイートプロジェクト (.eh3d) を開くか保存してください。", "打开或保存套件项目 (.eh3d)，即可把数据手册和说明书保存在其中。", "Abra ou guarde um projeto da suite (.eh3d) para guardar com ele folhas de dados e manuais."];
    NoAttachments: ["The project has no attachments yet.", "El proyecto todavía no tiene adjuntos.", "Das Projekt hat noch keine Anhänge.", "Le projet n’a pas encore de pièces jointes.", "Il progetto non ha ancora allegati.", "このプロジェクトにはまだ添付ファイルがありません。", "该项目还没有附件。", "O projeto ainda não tem anexos."];
    AttachFile: ["Attach a file…", "Adjuntar un archivo…", "Datei anhängen…", "Joindre un fichier…", "Allega un file…", "ファイルを添付…", "添加文件…", "Anexar um ficheiro…"];
    AttachLink: ["Attach a link…", "Adjuntar un enlace…", "Link anhängen…", "Joindre un lien…", "Allega un collegamento…", "リンクを添付…", "添加链接…", "Anexar uma ligação…"];
    ReadAgain: ["Read again", "Volver a leer", "Neu einlesen", "Relire", "Rileggi", "再読み込み", "重新读取", "Ler novamente"];
    Working: ["Working…", "Trabajando…", "Wird bearbeitet…", "Traitement…", "Elaborazione…", "処理中…", "正在处理…", "A trabalhar…"];
    Address: ["Address (https://…)", "Dirección (https://…)", "Adresse (https://…)", "Adresse (https://…)", "Indirizzo (https://…)", "アドレス (https://…)", "地址 (https://…)", "Endereço (https://…)"];
    Hash: ["SHA-256 of the file", "SHA-256 del archivo", "SHA-256 der Datei", "SHA-256 du fichier", "SHA-256 del file", "ファイルの SHA-256", "文件的 SHA-256", "SHA-256 do ficheiro"];
    HashFromFile: ["Take it from a file I have…", "Tomarlo de un archivo que tengo…", "Aus einer vorhandenen Datei übernehmen…", "Le prendre d’un fichier que j’ai…", "Prendilo da un file che ho…", "手元のファイルから取得…", "从我已有的文件获取…", "Obter de um ficheiro que tenho…"];
    Add: ["Add", "Añadir", "Hinzufügen", "Ajouter", "Aggiungi", "追加", "添加", "Adicionar"];
    Cancel: ["Cancel", "Cancelar", "Abbrechen", "Annuler", "Annulla", "キャンセル", "取消", "Cancelar"];
    Continue: ["Continue", "Continuar", "Fortfahren", "Continuer", "Continua", "続行", "继续", "Continuar"];
    Extract: ["Extract…", "Extraer…", "Extrahieren…", "Extraire…", "Estrai…", "取り出す…", "提取…", "Extrair…"];
    ExtractHere: ["Extract into this folder", "Extraer en esta carpeta", "In diesen Ordner extrahieren", "Extraire dans ce dossier", "Estrai in questa cartella", "このフォルダーに取り出す", "提取到此文件夹", "Extrair para esta pasta"];
    Remove: ["Remove from the project", "Quitar del proyecto", "Aus dem Projekt entfernen", "Retirer du projet", "Rimuovi dal progetto", "プロジェクトから削除", "从项目中移除", "Remover do projeto"];
    AskRemove: ["Remove {0} from the project? A file kept inside it is deleted.", "¿Quitar {0} del proyecto? Si el archivo está dentro, se borra.", "{0} aus dem Projekt entfernen? Eine darin gespeicherte Datei wird gelöscht.", "Retirer {0} du projet ? Un fichier conservé à l’intérieur est supprimé.", "Rimuovere {0} dal progetto? Un file conservato al suo interno viene eliminato.", "{0} をプロジェクトから削除しますか？内部に保存されたファイルは消去されます。", "要从项目中移除 {0} 吗？保存在项目内的文件会被删除。", "Remover {0} do projeto? Um ficheiro guardado dentro dele é apagado."];
    AskRemoveParts: ["It is linked to {0} component(s).", "Está ligado a {0} componente(s).", "Er ist mit {0} Bauteil(en) verknüpft.", "Elle est liée à {0} composant(s).", "È collegato a {0} componente/i.", "{0} 件の部品に関連付けられています。", "它关联了 {0} 个元件。", "Está ligado a {0} componente(s)."];
    AskExtract: ["{0} needs care: it is {1}. Extract it only if you trust who sent it.", "{0} requiere cuidado: es {1}. Extráelo solo si confías en quien lo envió.", "{0} erfordert Vorsicht: es ist {1}. Nur extrahieren, wenn Sie dem Absender vertrauen.", "{0} demande de la prudence : c’est {1}. Extrayez-le seulement si vous faites confiance à l’expéditeur.", "{0} richiede attenzione: è {1}. Estrailo solo se ti fidi di chi lo ha inviato.", "{0} には注意が必要です（{1}）。送信者を信頼できる場合にのみ取り出してください。", "{0} 需要谨慎：它是{1}。仅当您信任发送者时才提取。", "{0} requer cuidado: é {1}. Extraia-o apenas se confiar em quem o enviou."];
    Marked: ["Kept and marked: it is {0}. Nothing of it is ever run.", "Guardado y marcado: es {0}. Nada de él se ejecuta nunca.", "Gespeichert und markiert: es ist {0}. Nichts davon wird je ausgeführt.", "Conservé et signalé : c’est {0}. Rien n’en est jamais exécuté.", "Conservato e segnalato: è {0}. Nulla di esso viene mai eseguito.", "保存して印を付けました（{0}）。中身が実行されることはありません。", "已保存并标记：它是{0}。其中任何内容都不会被执行。", "Guardado e assinalado: é {0}. Nada dele é alguma vez executado."];
    Attached: ["Attached.", "Adjuntado.", "Angehängt.", "Joint.", "Allegato.", "添付しました。", "已添加。", "Anexado."];
    AlreadyThere: ["It was in the project already: it is stored only once.", "Ya estaba en el proyecto: se guarda una sola vez.", "Es war schon im Projekt: es wird nur einmal gespeichert.", "Il était déjà dans le projet : il n’est conservé qu’une fois.", "Era già nel progetto: viene conservato una sola volta.", "すでにプロジェクトにあります。保存は一度だけです。", "项目中已有此文件：只保存一份。", "Já estava no projeto: é guardado uma só vez."];
    Extracted: ["Written to {0}", "Escrito en {0}", "Geschrieben nach {0}", "Écrit dans {0}", "Scritto in {0}", "{0} に書き出しました", "已写入 {0}", "Escrito em {0}"];
    Removed: ["Removed from the project.", "Quitado del proyecto.", "Aus dem Projekt entfernt.", "Retiré du projet.", "Rimosso dal progetto.", "プロジェクトから削除しました。", "已从项目中移除。", "Removido do projeto."];
    Changed: ["The components were changed.", "Se cambiaron los componentes.", "Die Bauteile wurden geändert.", "Les composants ont été modifiés.", "I componenti sono stati modificati.", "部品の関連付けを変更しました。", "元件关联已更改。", "Os componentes foram alterados."];
    Failed: ["Not done: {0}", "No se hizo: {0}", "Nicht ausgeführt: {0}", "Non effectué : {0}", "Non eseguito: {0}", "実行できませんでした：{0}", "未完成：{0}", "Não foi feito: {0}"];
    HoldsCredential: ["{0} holds a token or password of this computer and is not put into a project, which is a file that is passed on.", "{0} contiene un token o una contraseña de este equipo y no se mete en un proyecto, que es un archivo que se comparte.", "{0} enthält ein Token oder Kennwort dieses Rechners und kommt nicht in ein Projekt, das weitergegeben wird.", "{0} contient un jeton ou un mot de passe de cet ordinateur et n’est pas mis dans un projet, qui est un fichier que l’on transmet.", "{0} contiene un token o una password di questo computer e non viene messo in un progetto, che è un file che si condivide.", "{0} にはこのコンピューターのトークンまたはパスワードが含まれるため、受け渡されるプロジェクトには入れません。", "{0} 含有本机的令牌或密码，不会放入会被分享的项目中。", "{0} contém um token ou uma palavra-passe deste computador e não é colocado num projeto, que é um ficheiro partilhado."];
    Inside: ["inside", "dentro", "enthalten", "inclus", "incluso", "内部", "内置", "dentro"];
    Link: ["link", "enlace", "Link", "lien", "collegamento", "リンク", "链接", "ligação"];
    LinkNote: ["Only a link: nothing is downloaded by itself.", "Solo un enlace: nada se descarga solo.", "Nur ein Link: nichts wird von selbst heruntergeladen.", "Un lien seulement : rien n’est téléchargé tout seul.", "Solo un collegamento: nulla viene scaricato da solo.", "リンクのみです。自動ではダウンロードされません。", "只是链接：不会自动下载任何内容。", "Apenas uma ligação: nada é transferido por si só."];
    CopyAddress: ["Copy the address", "Copiar la dirección", "Adresse kopieren", "Copier l’adresse", "Copia l’indirizzo", "アドレスをコピー", "复制地址", "Copiar o endereço"];
    NeedsCare: ["asks before it is taken out", "pregunta antes de sacarlo", "fragt vor der Entnahme", "demande avant extraction", "chiede prima di estrarlo", "取り出し前に確認", "取出前会询问", "pergunta antes de sair"];
    Components: ["Components", "Componentes", "Bauteile", "Composants", "Componenti", "部品", "元件", "Componentes"];
    Designator: ["Designator (R5)", "Referencia (R5)", "Bezeichner (R5)", "Repère (R5)", "Riferimento (R5)", "部品番号記号 (R5)", "位号 (R5)", "Referência (R5)"];
    Value: ["Value (10k)", "Valor (10k)", "Wert (10k)", "Valeur (10k)", "Valore (10k)", "値 (10k)", "数值 (10k)", "Valor (10k)"];
    PartNumber: ["Part number", "Número de pieza", "Teilenummer", "Référence fabricant", "Codice del pezzo", "型番", "型号", "Número de peça"];
    Maker: ["Maker (optional)", "Fabricante (opcional)", "Hersteller (optional)", "Fabricant (facultatif)", "Produttore (facoltativo)", "メーカー（任意）", "制造商（可选）", "Fabricante (opcional)"];
    LinkComponent: ["Link to the component", "Ligar al componente", "Mit dem Bauteil verknüpfen", "Lier au composant", "Collega al componente", "部品に関連付ける", "关联到元件", "Ligar ao componente"];
    Unlink: ["Unlink", "Desligar", "Lösen", "Délier", "Scollega", "解除", "解除关联", "Desligar"];
    FilterOf: ["Attachments of {0}", "Adjuntos de {0}", "Anhänge von {0}", "Pièces jointes de {0}", "Allegati di {0}", "{0} の添付ファイル", "{0} 的附件", "Anexos de {0}"];
    ShowAll: ["Show all", "Ver todos", "Alle anzeigen", "Tout afficher", "Mostra tutti", "すべて表示", "显示全部", "Mostrar todos"];
    ChooseFile: ["Choose a file", "Elegir un archivo", "Datei wählen", "Choisir un fichier", "Scegli un file", "ファイルを選択", "选择文件", "Escolher um ficheiro"];
    ChooseFolder: ["Choose a folder", "Elegir una carpeta", "Ordner wählen", "Choisir un dossier", "Scegli una cartella", "フォルダーを選択", "选择文件夹", "Escolher uma pasta"];
    SaveAs: ["Save a copy as", "Guardar una copia como", "Kopie speichern unter", "Enregistrer une copie sous", "Salva una copia come", "コピーの保存先", "副本另存为", "Guardar uma cópia como"];
    Open: ["Open", "Abrir", "Öffnen", "Ouvrir", "Apri", "開く", "打开", "Abrir"];
    Save: ["Save", "Guardar", "Speichern", "Enregistrer", "Salva", "保存", "保存", "Guardar"];
    Up: ["Up", "Subir", "Nach oben", "Remonter", "Su", "上へ", "上一级", "Acima"];
    Desktop: ["Desktop", "Escritorio", "Desktop", "Bureau", "Desktop", "デスクトップ", "桌面", "Ambiente de trabalho"];
    Documents: ["Documents", "Documentos", "Dokumente", "Documents", "Documenti", "ドキュメント", "文档", "Documentos"];
    Downloads: ["Downloads", "Descargas", "Downloads", "Téléchargements", "Download", "ダウンロード", "下载", "Transferências"];
    ProjectFolder: ["Folder of the project", "Carpeta del proyecto", "Projektordner", "Dossier du projet", "Cartella del progetto", "プロジェクトのフォルダー", "项目文件夹", "Pasta do projeto"];
}

/// The text in a language.
pub fn tr(lang: Lang, text: Text) -> &'static str {
    text.words()[column(lang)]
}

/// The language of a code (`es`, `pt-br`); English for a code the suite does not have.
pub fn lang_of(code: &str) -> Lang {
    Lang::from_code(code).unwrap_or(Lang::En)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_is_there_in_the_eight_languages_with_the_same_holes() {
        for text in Text::ALL {
            for lang in Lang::ALL {
                let said = tr(lang, *text);
                assert!(!said.trim().is_empty(), "{text:?} {lang:?}");
                for hole in ["{0}", "{1}"] {
                    assert_eq!(
                        said.contains(hole),
                        tr(Lang::En, *text).contains(hole),
                        "{text:?} {lang:?} {hole}"
                    );
                }
            }
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!tr(lang, *text).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn a_code_gives_its_language_and_english_for_the_rest() {
        assert_eq!(lang_of("pt-br"), Lang::Pt);
        assert_eq!(lang_of("xx"), Lang::En);
    }
}
