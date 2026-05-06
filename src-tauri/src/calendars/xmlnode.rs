use crate::error::{AppError, AppResult};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Lightweight XML tree representation. Names are local (the namespace prefix is stripped
/// before the colon) so we can match `D:href`, `d:href`, `href`, etc. uniformly.
#[derive(Debug, Default, Clone)]
pub struct XmlNode {
    pub name: String,
    pub text: String,
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    pub fn parse(xml: &str) -> AppResult<XmlNode> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut root = XmlNode {
            name: "_root".into(),
            ..Default::default()
        };
        let mut stack: Vec<XmlNode> = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader
                .read_event_into(&mut buf)
                .map_err(|e| AppError::CalDav(format!("XML parse error: {e}")))?
            {
                Event::Start(e) => {
                    let name = local_name(e.name().as_ref());
                    stack.push(XmlNode {
                        name,
                        ..Default::default()
                    });
                }
                Event::End(_) => {
                    let node = stack.pop().unwrap_or_default();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        root.children.push(node);
                    }
                }
                Event::Empty(e) => {
                    let name = local_name(e.name().as_ref());
                    let node = XmlNode {
                        name,
                        ..Default::default()
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        root.children.push(node);
                    }
                }
                Event::Text(t) => {
                    if let Some(top) = stack.last_mut() {
                        let s = t
                            .unescape()
                            .map_err(|e| AppError::CalDav(format!("text decode: {e}")))?;
                        top.text.push_str(&s);
                    }
                }
                Event::CData(t) => {
                    if let Some(top) = stack.last_mut() {
                        top.text
                            .push_str(std::str::from_utf8(t.as_ref()).unwrap_or(""));
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(root)
    }

    /// First descendant with the given local name (depth-first).
    pub fn find(&self, name: &str) -> Option<&XmlNode> {
        for c in &self.children {
            if c.name.eq_ignore_ascii_case(name) {
                return Some(c);
            }
            if let Some(n) = c.find(name) {
                return Some(n);
            }
        }
        None
    }

    /// All children at any depth with the given local name (depth-first).
    pub fn find_all<'a>(&'a self, name: &str, out: &mut Vec<&'a XmlNode>) {
        for c in &self.children {
            if c.name.eq_ignore_ascii_case(name) {
                out.push(c);
            }
            c.find_all(name, out);
        }
    }

    pub fn has_child(&self, name: &str) -> bool {
        self.children
            .iter()
            .any(|c| c.name.eq_ignore_ascii_case(name))
    }
}

fn local_name(raw: &[u8]) -> String {
    let s = std::str::from_utf8(raw).unwrap_or("");
    match s.split_once(':') {
        Some((_, local)) => local.to_string(),
        None => s.to_string(),
    }
}
