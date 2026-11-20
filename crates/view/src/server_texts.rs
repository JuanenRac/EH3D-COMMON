//! The words of the settings of the server, the state of the link and the login window, in the eight languages of the project, so that every
//! machine program shows the same thing in the same words.
//!
//! Each row has the eight texts in the order English, Spanish, Italian, German, French, Portuguese (of Portugal), Chinese, Japanese (the type of the
//! table makes the compiler refuse a row with fewer or more). A language that has no words of its own for these texts (the other 29 of
//! [`Lang::ALL`]) speaks its main language when it is a variant (`pt-br`, `fr-ca`) and English otherwise.

use crate::texts::{Lang, CORE_LANGS};

macro_rules! server_texts {
    ($($name:ident => [$($t:expr),+ $(,)?]),+ $(,)?) => {
        /// A word or sentence of the settings of the server.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ServerText { $($name),+ }
        /// Every text, in the order of the table.
        pub const ALL_SERVER_TEXTS: &[ServerText] = &[$(ServerText::$name),+];
        const TABLE: &[[&str; 8]] = &[$([$($t),+]),+];
        impl ServerText {
            fn index(self) -> usize {
                ALL_SERVER_TEXTS.iter().position(|t| *t == self).unwrap_or(0)
            }
            /// The name of the text, as the files of the languages would write it.
            pub fn name(self) -> &'static str {
                match self { $(ServerText::$name => stringify!($name)),+ }
            }
        }
    };
}

server_texts! {
    Title => ["Server", "Servidor", "Server", "Server", "Serveur", "Servidor", "服务器", "サーバー"],
    FieldAddress => ["Server address", "Dirección del servidor", "Indirizzo del server", "Serveradresse", "Adresse du serveur", "Endereço do servidor", "服务器地址", "サーバーのアドレス"],
    FieldToken => ["Token of the program", "Token del programa", "Token del programma", "Token des Programms", "Jeton du programme", "Token do programa", "程序令牌", "プログラムのトークン"],
    FieldMachine => ["Machine", "Máquina", "Macchina", "Maschine", "Machine", "Máquina", "机器", "機械"],
    FieldTimeout => ["Time limit of a request (seconds)", "Tiempo máximo de una petición (segundos)", "Tempo massimo di una richiesta (secondi)", "Zeitlimit einer Anfrage (Sekunden)", "Durée maximale d'une requête (secondes)", "Tempo máximo de um pedido (segundos)", "单次请求的时间上限（秒）", "1回の要求の制限時間（秒）"],
    FieldRetries => ["Retries after a network failure", "Reintentos tras un fallo de red", "Nuovi tentativi dopo un guasto di rete", "Wiederholungen nach einem Netzwerkfehler", "Nouvelles tentatives après une panne réseau", "Novas tentativas após uma falha de rede", "网络故障后的重试次数", "ネットワーク障害後の再試行回数"],
    FieldMdns => ["Look for the server on the local network", "Buscar el servidor en la red local", "Cerca il server nella rete locale", "Server im lokalen Netzwerk suchen", "Chercher le serveur sur le réseau local", "Procurar o servidor na rede local", "在本地网络中查找服务器", "ローカルネットワークでサーバーを探す"],
    TokenKept => ["A token is saved; it is never shown", "Hay un token guardado; nunca se muestra", "Un token è salvato; non viene mai mostrato", "Ein Token ist gespeichert; er wird nie angezeigt", "Un jeton est enregistré ; il n'est jamais affiché", "Há um token guardado; nunca é mostrado", "已保存令牌；不会显示", "トークンは保存済みです。表示されることはありません"],
    TokenNone => ["No token", "Sin token", "Nessun token", "Kein Token", "Aucun jeton", "Sem token", "无令牌", "トークンなし"],
    TokenRemove => ["Remove the token", "Quitar el token", "Rimuovi il token", "Token entfernen", "Supprimer le jeton", "Remover o token", "移除令牌", "トークンを削除"],
    TokenWillRemove => ["The token will be removed when you apply", "El token se quitará al aplicar", "Il token verrà rimosso all'applicazione", "Der Token wird beim Übernehmen entfernt", "Le jeton sera supprimé à l'application", "O token será removido ao aplicar", "应用时将移除令牌", "適用するとトークンが削除されます"],
    Apply => ["Apply", "Aplicar", "Applica", "Übernehmen", "Appliquer", "Aplicar", "应用", "適用"],
    Undo => ["Undo changes", "Deshacer cambios", "Annulla le modifiche", "Änderungen verwerfen", "Annuler les modifications", "Anular alterações", "撤销更改", "変更を元に戻す"],
    TestConnection => ["Test the connection", "Probar la conexión", "Prova la connessione", "Verbindung testen", "Tester la connexion", "Testar a ligação", "测试连接", "接続をテスト"],
    Login => ["Log in", "Iniciar sesión", "Accedi", "Anmelden", "Se connecter", "Iniciar sessão", "登录", "ログイン"],
    Logout => ["Log out", "Cerrar sesión", "Esci", "Abmelden", "Se déconnecter", "Terminar sessão", "退出登录", "ログアウト"],
    LoginUser => ["User", "Usuario", "Utente", "Benutzer", "Utilisateur", "Utilizador", "用户", "ユーザー"],
    LoginPassword => ["Password", "Contraseña", "Password", "Passwort", "Mot de passe", "Palavra-passe", "密码", "パスワード"],
    LoginCancel => ["Cancel", "Cancelar", "Annulla", "Abbrechen", "Annuler", "Cancelar", "取消", "キャンセル"],
    LoggedInAs => ["Logged in as", "Sesión iniciada como", "Accesso effettuato come", "Angemeldet als", "Connecté en tant que", "Sessão iniciada como", "当前登录用户", "ログイン中のユーザー"],
    LinkConnected => ["Connected", "Conectado", "Connesso", "Verbunden", "Connecté", "Ligado", "已连接", "接続済み"],
    LinkDisconnected => ["Disconnected", "Desconectado", "Disconnesso", "Getrennt", "Déconnecté", "Desligado", "已断开", "切断"],
    LinkSessionExpired => ["Session expired", "Sesión caducada", "Sessione scaduta", "Sitzung abgelaufen", "Session expirée", "Sessão expirada", "会话已过期", "セッション期限切れ"],
    LinkNoSession => ["No session", "Sin sesión", "Nessuna sessione", "Keine Sitzung", "Aucune session", "Sem sessão", "无会话", "セッションなし"],
    LinkUnknown => ["Not checked yet", "Aún sin comprobar", "Non ancora verificato", "Noch nicht geprüft", "Pas encore vérifié", "Ainda não verificado", "尚未检查", "未確認"],
    LinkNotSet => ["No server set", "Sin servidor configurado", "Nessun server impostato", "Kein Server eingestellt", "Aucun serveur défini", "Sem servidor definido", "未设置服务器", "サーバー未設定"],
    ProblemAddress => ["must be host:port, for example 192.168.0.180:18200", "debe ser host:puerto, por ejemplo 192.168.0.180:18200", "deve essere host:porta, ad esempio 192.168.0.180:18200", "muss Host:Port sein, zum Beispiel 192.168.0.180:18200", "doit être hôte:port, par exemple 192.168.0.180:18200", "tem de ser anfitrião:porta, por exemplo 192.168.0.180:18200", "必须是 主机:端口，例如 192.168.0.180:18200", "ホスト:ポートの形式で入力してください（例: 192.168.0.180:18200）"],
    ProblemSpaces => ["must not hold spaces or control characters", "no puede tener espacios ni caracteres de control", "non può contenere spazi o caratteri di controllo", "darf keine Leerzeichen oder Steuerzeichen enthalten", "ne doit contenir ni espaces ni caractères de contrôle", "não pode ter espaços nem caracteres de controlo", "不能包含空格或控制字符", "空白や制御文字は使えません"],
    ProblemRange => ["must be a whole number from {lo} to {hi}", "debe ser un número entero de {lo} a {hi}", "deve essere un numero intero da {lo} a {hi}", "muss eine ganze Zahl von {lo} bis {hi} sein", "doit être un nombre entier de {lo} à {hi}", "tem de ser um número inteiro de {lo} a {hi}", "必须是 {lo} 到 {hi} 之间的整数", "{lo} から {hi} までの整数で入力してください"],
    ErrNoAddress => ["No server address is set", "No hay dirección de servidor", "Nessun indirizzo del server impostato", "Keine Serveradresse eingestellt", "Aucune adresse de serveur définie", "Não há endereço de servidor definido", "未设置服务器地址", "サーバーのアドレスが設定されていません"],
    ErrNoMachine => ["No machine is chosen", "No hay máquina elegida", "Nessuna macchina scelta", "Keine Maschine gewählt", "Aucune machine choisie", "Nenhuma máquina escolhida", "未选择机器", "機械が選択されていません"],
    ErrMdnsOff => ["Searching the network is off: turn it on in the settings", "La búsqueda en la red está desactivada: actívala en los ajustes", "La ricerca in rete è disattivata: attivala nelle impostazioni", "Die Netzwerksuche ist aus: In den Einstellungen einschalten", "La recherche sur le réseau est désactivée : activez-la dans les réglages", "A pesquisa na rede está desativada: ative-a nas definições", "网络搜索已关闭：请在设置中开启", "ネットワーク検索はオフです。設定でオンにしてください"],
    ErrNoResolver => ["This program cannot search the network", "Este programa no puede buscar en la red", "Questo programma non può cercare in rete", "Dieses Programm kann das Netzwerk nicht durchsuchen", "Ce programme ne peut pas chercher sur le réseau", "Este programa não pode pesquisar na rede", "此程序无法搜索网络", "このプログラムはネットワークを検索できません"],
    ErrBadReply => ["The server answered something this program cannot read", "El servidor respondió algo que este programa no puede leer", "Il server ha risposto con qualcosa che questo programma non sa leggere", "Der Server hat etwas geantwortet, das dieses Programm nicht lesen kann", "Le serveur a répondu quelque chose que ce programme ne sait pas lire", "O servidor respondeu algo que este programa não consegue ler", "服务器返回了此程序无法读取的内容", "サーバーの応答をこのプログラムは読み取れません"],
    Attempts => ["tries", "intentos", "tentativi", "Versuche", "essais", "tentativas", "次尝试", "回試行"],
    Applied => ["Settings applied", "Ajustes aplicados", "Impostazioni applicate", "Einstellungen übernommen", "Réglages appliqués", "Definições aplicadas", "设置已应用", "設定を適用しました"],
    HealthOk => ["The server answered", "El servidor respondió", "Il server ha risposto", "Der Server hat geantwortet", "Le serveur a répondu", "O servidor respondeu", "服务器已响应", "サーバーが応答しました"],
    NotApplied => ["There are changes not applied yet", "Hay cambios sin aplicar", "Ci sono modifiche non ancora applicate", "Es gibt noch nicht übernommene Änderungen", "Des modifications ne sont pas encore appliquées", "Há alterações por aplicar", "有尚未应用的更改", "未適用の変更があります"],
}

/// The text in a language. A language that has no words of its own here speaks its main language (a variant) or English.
pub fn tr(lang: Lang, text: ServerText) -> &'static str {
    let row = &TABLE[text.index()];
    let mut l = lang;
    loop {
        let i = Lang::ALL.iter().position(|x| *x == l).unwrap_or(0);
        if i < CORE_LANGS {
            return row[i];
        }
        match l.fallback() {
            Some(base) => l = base,
            None => return row[0],
        }
    }
}

/// The text with `{lo}` and `{hi}` filled in (the range of a number).
pub fn range_text(lang: Lang, lo: u64, hi: u64) -> String {
    tr(lang, ServerText::ProblemRange)
        .replace("{lo}", &lo.to_string())
        .replace("{hi}", &hi.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
    ];

    #[test]
    fn every_text_has_its_eight_values_and_none_is_blank_or_a_placeholder() {
        for &t in ALL_SERVER_TEXTS {
            for l in CORE {
                let v = tr(l, t);
                assert!(!v.trim().is_empty(), "{t:?} in {l:?}");
                assert_eq!(v, v.trim(), "{t:?} in {l:?}");
                assert!(!v.contains('\t') && !v.contains('\n'), "{t:?} in {l:?}");
                assert!(!v.contains("TODO") && !v.contains("???"), "{t:?} in {l:?}");
            }
        }
        assert_eq!(TABLE.len(), ALL_SERVER_TEXTS.len());
        assert!(ALL_SERVER_TEXTS.len() >= 38);
    }

    #[test]
    fn the_names_are_unique() {
        let mut names: Vec<&str> = ALL_SERVER_TEXTS.iter().map(|t| t.name()).collect();
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n);
    }

    #[test]
    fn a_sentence_is_translated_in_every_language() {
        // only a proper name or a loan word may be the English text: no more than three of the eight for a text
        for &t in ALL_SERVER_TEXTS {
            let en = tr(Lang::En, t);
            let same = CORE.iter().filter(|&&l| tr(l, t) == en).count();
            assert!(same <= 4, "{t:?} is the English text in {same} languages");
        }
        for t in [
            ServerText::ProblemAddress,
            ServerText::ErrBadReply,
            ServerText::TokenKept,
            ServerText::FieldTimeout,
        ] {
            for l in CORE.into_iter().skip(1) {
                assert_ne!(tr(l, t), tr(Lang::En, t), "{t:?} in {l:?}");
            }
        }
    }

    #[test]
    fn chinese_and_japanese_use_their_own_letters() {
        for &t in ALL_SERVER_TEXTS {
            for l in [Lang::Zh, Lang::Ja] {
                assert!(
                    tr(l, t).chars().any(|c| (c as u32) > 0x2e80),
                    "{t:?} in {l:?}"
                );
            }
        }
    }

    #[test]
    fn the_markers_are_the_same_in_every_language() {
        for &t in ALL_SERVER_TEXTS {
            let en = tr(Lang::En, t);
            for l in CORE {
                let v = tr(l, t);
                for marker in ["{lo}", "{hi}"] {
                    assert_eq!(
                        v.matches(marker).count(),
                        en.matches(marker).count(),
                        "{t:?} in {l:?}"
                    );
                }
                assert_eq!(
                    v.matches('{').count(),
                    v.matches('}').count(),
                    "{t:?} in {l:?}"
                );
            }
        }
    }

    #[test]
    fn the_portuguese_is_the_one_of_portugal() {
        assert!(tr(Lang::Pt, ServerText::LoginPassword).contains("Palavra-passe"));
        assert!(tr(Lang::Pt, ServerText::Login).contains("sessão"));
        assert!(tr(Lang::Pt, ServerText::FieldAddress).contains("Endereço"));
        assert!(tr(Lang::Pt, ServerText::TestConnection).contains("ligação"));
        assert!(tr(Lang::Pt, ServerText::ProblemSpaces).contains("controlo"));
        assert!(tr(Lang::Pt, ServerText::LoginUser).contains("Utilizador"));
        for &t in ALL_SERVER_TEXTS {
            let v = tr(Lang::Pt, t);
            for brazilian in ["você", "arquivo", "tela", "celular", "senha", "conexão"] {
                assert!(!v.to_lowercase().contains(brazilian), "{t:?}: {v}");
            }
        }
    }

    #[test]
    fn the_other_languages_speak_their_main_language_or_english() {
        for &t in ALL_SERVER_TEXTS {
            assert_eq!(tr(Lang::PtBr, t), tr(Lang::Pt, t));
            assert_eq!(tr(Lang::FrCa, t), tr(Lang::Fr, t));
            assert_eq!(tr(Lang::Bg, t), tr(Lang::En, t));
        }
    }

    #[test]
    fn the_range_is_written_with_its_numbers() {
        assert_eq!(
            range_text(Lang::En, 1, 120),
            "must be a whole number from 1 to 120"
        );
        for l in CORE {
            let s = range_text(l, 0, 5);
            assert!(
                s.contains('0') && s.contains('5') && !s.contains('{'),
                "{l:?}"
            );
        }
    }
}
