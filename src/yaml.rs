//! A tiny YAML tree with source positions, built on `yaml-rust2` events,
//! plus a comment scanner (YAML parsers discard comments, but we need them
//! for `# frozen: vX` markers and inline ignores).

use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// A YAML node with its 1-based start position and the last line it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub value: Value,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Scalar text and whether it was written as a plain (unquoted) scalar.
    Scalar(String, bool),
    Seq(Vec<Node>),
    Map(Vec<(Node, Node)>),
    /// An alias (`*anchor`); we do not resolve aliases.
    Alias,
}

impl Node {
    /// Scalar text, if this node is a non-null scalar.
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Scalar(s, _) if !self.is_null() => Some(s),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(&self.value, Value::Scalar(s, true) if matches!(s.as_str(), "" | "~" | "null" | "Null" | "NULL"))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            Value::Scalar(s, true) => match s.as_str() {
                "true" | "True" | "TRUE" => Some(true),
                "false" | "False" | "FALSE" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Seq(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(Node, Node)]> {
        match &self.value {
            Value::Map(v) => Some(v),
            _ => None,
        }
    }

    /// Look up a key in a mapping node.
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entry(key).map(|(_, v)| v)
    }

    /// Look up a key in a mapping node, returning both key and value nodes.
    pub fn entry(&self, key: &str) -> Option<(&Node, &Node)> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .map(|(k, v)| (k, v))
    }

    /// Scalar items of a sequence node (or a single scalar treated as one item).
    pub fn str_items(&self) -> Vec<&Node> {
        match &self.value {
            Value::Seq(v) => v.iter().filter(|n| n.as_str().is_some()).collect(),
            Value::Scalar(..) if !self.is_null() => vec![self],
            _ => vec![],
        }
    }
}

/// A YAML syntax error.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Default)]
struct Builder {
    /// Open containers: (frame, line, col, anchor id).
    stack: Vec<(Frame, usize, usize, usize)>,
    anchors: std::collections::HashMap<usize, Node>,
    root: Option<Node>,
    docs: usize,
}

enum Frame {
    Seq(Vec<Node>),
    Map(Vec<Node>),
}

impl Builder {
    fn push_node(&mut self, node: Node, anchor: usize) {
        if anchor > 0 {
            self.anchors.insert(anchor, node.clone());
        }
        match self.stack.last_mut() {
            Some((Frame::Seq(v), ..)) | Some((Frame::Map(v), ..)) => v.push(node),
            None => {
                if self.root.is_none() && self.docs <= 1 {
                    self.root = Some(node);
                }
            }
        }
    }
}

fn pos(m: &Marker) -> (usize, usize) {
    // yaml-rust2 lines are 1-based; columns are 0-based.
    (m.line().max(1), m.col() + 1)
}

impl MarkedEventReceiver for Builder {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        let (line, col) = pos(&mark);
        match ev {
            Event::DocumentStart => self.docs += 1,
            Event::Scalar(s, style, anchor, _) => self.push_node(
                Node {
                    value: Value::Scalar(s, style == TScalarStyle::Plain),
                    line,
                    col,
                    end_line: line,
                },
                anchor,
            ),
            Event::Alias(id) => {
                // Resolve the alias to a copy of the anchored node, reported
                // at the alias position.
                let value = self
                    .anchors
                    .get(&id)
                    .map(|n| {
                        let mut n = n.clone();
                        relocate(&mut n, line, col);
                        n.value
                    })
                    .unwrap_or(Value::Alias);
                self.push_node(
                    Node {
                        value,
                        line,
                        col,
                        end_line: line,
                    },
                    0,
                )
            }
            Event::SequenceStart(anchor, _) => {
                self.stack.push((Frame::Seq(vec![]), line, col, anchor))
            }
            Event::MappingStart(anchor, _) => {
                self.stack.push((Frame::Map(vec![]), line, col, anchor))
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let Some((frame, line, col, anchor)) = self.stack.pop() else {
                    return;
                };
                let (value, end_line) = match frame {
                    Frame::Seq(items) => {
                        let end = items.iter().map(|n| n.end_line).max().unwrap_or(line);
                        (Value::Seq(items), end)
                    }
                    Frame::Map(items) => {
                        let end = items.iter().map(|n| n.end_line).max().unwrap_or(line);
                        let mut pairs = Vec::new();
                        let mut it = items.into_iter();
                        while let (Some(k), Some(v)) = (it.next(), it.next()) {
                            pairs.push((k, v));
                        }
                        (Value::Map(pairs), end)
                    }
                };
                self.push_node(
                    Node {
                        value,
                        line,
                        col,
                        end_line,
                    },
                    anchor,
                );
            }
            _ => {}
        }
    }
}

/// Move a node and all its children to one position (used for aliases).
fn relocate(n: &mut Node, line: usize, col: usize) {
    n.line = line;
    n.col = col;
    n.end_line = line;
    match &mut n.value {
        Value::Seq(items) => items.iter_mut().for_each(|c| relocate(c, line, col)),
        Value::Map(pairs) => pairs.iter_mut().for_each(|(k, v)| {
            relocate(k, line, col);
            relocate(v, line, col);
        }),
        _ => {}
    }
}

/// Parse the first YAML document of `src`. Returns `Ok(None)` for an empty document.
pub fn parse(src: &str) -> Result<Option<Node>, ParseError> {
    let mut b = Builder::default();
    let mut parser = Parser::new_from_str(src);
    parser.load(&mut b, true).map_err(|e| {
        let (line, col) = pos(e.marker());
        ParseError {
            message: e.info().to_string(),
            line,
            col,
        }
    })?;
    Ok(b.root.filter(|n| !n.is_null()))
}

/// A `#` comment found in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// 1-based line.
    pub line: usize,
    /// Text after the `#`.
    pub text: String,
    /// True when the comment is the only thing on its line.
    pub own_line: bool,
}

/// Find comments line by line. This is a pragmatic scanner: it tracks quotes
/// on a single line and treats `#` as a comment start when it begins the line
/// or follows whitespace, which matches YAML's rules for plain scalars.
pub fn comments(src: &str) -> Vec<Comment> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let mut quote: Option<char> = None;
        let mut prev = ' ';
        for (idx, c) in line.char_indices() {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if (c == '"' || c == '\'')
                    && (prev == ' '
                        || prev == ':'
                        || prev == '-'
                        || prev == '['
                        || prev == ','
                        || idx == 0) =>
                {
                    quote = Some(c)
                }
                None if c == '#' && (idx == 0 || prev.is_whitespace()) => {
                    out.push(Comment {
                        line: i + 1,
                        text: line[idx + 1..].to_string(),
                        own_line: line[..idx].trim().is_empty(),
                    });
                    break;
                }
                None => {}
            }
            prev = c;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_one_based() {
        let n = parse("repos:\n  - repo: x\n    rev: v1\n")
            .unwrap()
            .unwrap();
        let repos = n.get("repos").unwrap().as_seq().unwrap();
        let (k, v) = repos[0].entry("rev").unwrap();
        assert_eq!((k.line, k.col), (3, 5));
        assert_eq!((v.line, v.col), (3, 10));
        assert_eq!(repos[0].end_line, 3);
        assert_eq!(n.end_line, 3);
    }

    #[test]
    fn parse_error_has_location() {
        let e = parse("repos:\n  - repo: x\n   rev: [\n").unwrap_err();
        assert!(e.line >= 2, "{e:?}");
    }

    #[test]
    fn resolves_aliases() {
        let n = parse("a: &deps [x, y]\nb: *deps\n").unwrap().unwrap();
        let b = n.get("b").unwrap();
        assert_eq!(b.as_seq().unwrap().len(), 2);
        assert_eq!(b.line, 2);
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(parse("").unwrap(), None);
        assert_eq!(parse("# only a comment\n").unwrap(), None);
    }

    #[test]
    fn finds_comments_outside_quotes() {
        let c = comments("a: 'x # y'  # real\n# own\nb: c#notcomment\n");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].text, " real");
        assert!(!c[0].own_line);
        assert!(c[1].own_line);
    }
}
