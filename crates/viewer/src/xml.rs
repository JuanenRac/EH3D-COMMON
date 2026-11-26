//! A very small reader of XML, enough for the text and tables of office documents. A document type
//! declaration is refused, so there are no entities of the file and nothing outside it is ever read;
//! only the five entities of XML and numeric references are replaced.

use crate::error::ViewerError;

/// One piece of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    /// A start tag; `empty` for `<a/>`, which gives no [`Piece::End`].
    Start {
        name: String,
        attributes: Vec<(String, String)>,
        empty: bool,
    },
    End(String),
    Text(String),
}

/// The value of an attribute of a start tag.
pub(crate) fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn damaged(why: &str) -> ViewerError {
    ViewerError::Damaged(format!("XML: {why}"))
}

fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.bytes().take(12).position(|b| b == b';');
        let replaced = end.and_then(|end| match &rest[1..end] {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            number if number.starts_with("#x") => u32::from_str_radix(&number[2..], 16)
                .ok()
                .and_then(char::from_u32),
            number if number.starts_with('#') => number[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        });
        match (replaced, end) {
            (Some(c), Some(end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Reads the pieces of a document one after another.
pub(crate) struct Reader<'a> {
    rest: &'a str,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Reader {
            rest: text.strip_prefix('\u{feff}').unwrap_or(text),
        }
    }

    /// Skips to just after `end`.
    fn skip_past(&mut self, end: &str) -> Result<&'a str, ViewerError> {
        let at = self.rest.find(end).ok_or_else(|| damaged("not closed"))?;
        let before = &self.rest[..at];
        self.rest = &self.rest[at + end.len()..];
        Ok(before)
    }

    /// The next piece, or `None` at the end.
    pub(crate) fn next_piece(&mut self) -> Result<Option<Piece>, ViewerError> {
        loop {
            if self.rest.is_empty() {
                return Ok(None);
            }
            if !self.rest.starts_with('<') {
                let at = self.rest.find('<').unwrap_or(self.rest.len());
                let text = decode(&self.rest[..at]);
                self.rest = &self.rest[at..];
                return Ok(Some(Piece::Text(text)));
            }
            if self.rest.starts_with("<!--") {
                self.rest = &self.rest[4..];
                self.skip_past("-->")?;
            } else if self.rest.starts_with("<![CDATA[") {
                self.rest = &self.rest[9..];
                let text = self.skip_past("]]>")?;
                return Ok(Some(Piece::Text(text.to_string())));
            } else if self.rest.starts_with("<!") {
                return Err(damaged("document type declarations are not accepted"));
            } else if self.rest.starts_with("<?") {
                self.skip_past("?>")?;
            } else if self.rest.starts_with("</") {
                self.rest = &self.rest[2..];
                let name = self.skip_past(">")?;
                return Ok(Some(Piece::End(name.trim().to_string())));
            } else {
                return self.start_tag().map(Some);
            }
        }
    }

    fn start_tag(&mut self) -> Result<Piece, ViewerError> {
        // the end of the tag is the first '>' outside quotes
        let mut quote = None;
        let mut end = None;
        for (at, c) in self.rest.char_indices().skip(1) {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => quote = Some(c),
                None if c == '>' => {
                    end = Some(at);
                    break;
                }
                None => {}
            }
        }
        let end = end.ok_or_else(|| damaged("tag not closed"))?;
        let mut body = &self.rest[1..end];
        self.rest = &self.rest[end + 1..];
        let empty = body.ends_with('/');
        if empty {
            body = &body[..body.len() - 1];
        }
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name = body[..name_end].to_string();
        if name.is_empty() {
            return Err(damaged("tag without a name"));
        }
        let mut attributes = Vec::new();
        let mut tail = body[name_end..].trim_start();
        while let Some(equals) = tail.find('=') {
            let key = tail[..equals].trim().to_string();
            let value = tail[equals + 1..].trim_start();
            let Some(q) = value.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                return Err(damaged("attribute without quotes"));
            };
            let value = &value[1..];
            let close = value
                .find(q)
                .ok_or_else(|| damaged("attribute not closed"))?;
            if attributes.len() < 64 {
                attributes.push((key, decode(&value[..close])));
            }
            tail = value[close + 1..].trim_start();
        }
        Ok(Piece::Start {
            name,
            attributes,
            empty,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(text: &str) -> Result<Vec<Piece>, ViewerError> {
        let mut reader = Reader::new(text);
        let mut out = Vec::new();
        while let Some(piece) = reader.next_piece()? {
            out.push(piece);
        }
        Ok(out)
    }

    #[test]
    fn tags_attributes_text_and_the_entities_of_xml() {
        let pieces =
            all("<?xml version=\"1.0\"?><a k='1 > 0'><!-- c --><b/>x &amp; &#65;&#x42; &no; &</a>")
                .unwrap();
        assert_eq!(
            pieces,
            vec![
                Piece::Start {
                    name: "a".into(),
                    attributes: vec![("k".into(), "1 > 0".into())],
                    empty: false
                },
                Piece::Start {
                    name: "b".into(),
                    attributes: vec![],
                    empty: true
                },
                Piece::Text("x & AB &no; &".into()),
                Piece::End("a".into()),
            ]
        );
        assert_eq!(
            all("<![CDATA[<x>]]>").unwrap(),
            vec![Piece::Text("<x>".into())]
        );
    }

    #[test]
    fn a_document_type_with_entities_is_refused() {
        let bomb = "<!DOCTYPE a [<!ENTITY x \"lol\"><!ENTITY y SYSTEM \"file:///etc/passwd\">]><a>&x;&y;</a>";
        assert!(matches!(all(bomb), Err(ViewerError::Damaged(_))));
    }

    #[test]
    fn broken_documents_are_errors_and_never_panic() {
        for text in [
            "<a",
            "<!-- x",
            "<a k=1>",
            "<a k=\"1>",
            "</",
            "<>",
            "<![CDATA[x",
        ] {
            assert!(all(text).is_err(), "{text}");
        }
        // every cut of a document with wide characters
        let text = "<a k=\"é\">ü &lt; 日本<!--x--></a>";
        for (at, _) in text.char_indices() {
            let _ = all(&text[..at]);
        }
    }
}
