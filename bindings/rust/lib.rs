//! This crate provides Gemtext language support for the [tree-sitter] parsing library.
//!
//! Typically, you will use the [`LANGUAGE`] constant to add this language to a
//! tree-sitter [`Parser`], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = tree_sitter_gemtext::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading Gemtext parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [`Parser`]: https://docs.rs/tree-sitter/0.26.3/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter::Node;
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    unsafe fn tree_sitter_gemtext() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for this grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_gemtext) };

/// The content of the [`node-types.json`] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(with_highlights_query)]
/// The syntax highlighting query for this grammar.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(with_injections_query)]
/// The language injection query for this grammar.
pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");

#[cfg(with_locals_query)]
/// The local variable query for this grammar.
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");

#[cfg(with_tags_query)]
/// The symbol tagging query for this grammar.
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

use std::{ops::Range, str::FromStr};

mod token_types;

pub use token_types::TokenType;

#[derive(Debug, Clone)]
pub struct Spanned<T>(pub T, pub Range<usize>);

pub type Token = Spanned<TokenType>;

impl Token {
    pub fn new(kind: TokenType, span: Range<usize>) -> Self {
        Spanned(kind, span)
    }

    pub fn kind(&self) -> &TokenType {
        &self.0
    }

    pub fn span(&self) -> &Range<usize> {
        &self.1
    }
}

impl TryFrom<&Node<'_>> for Token {
    type Error = ();
    fn try_from(node: &Node<'_>) -> Result<Self, Self::Error> {
        if let Ok(t) = TokenType::from_str(node.kind()) {
            let r = node.range();
            return Ok(Token::new(t, r.start_byte..r.end_byte));
        }
        Err(())
    }
}

pub fn tokenize(source: &str) -> Option<Vec<Token>> {
    let mut parser = tree_sitter::Parser::new();
    let language = LANGUAGE;
    parser.set_language(&language.into()).ok()?;
    let mut token_list = Vec::new();
    let tree = parser.parse(source, None)?;
    let mut parse_stack = vec![tree.root_node()];
    while let Some(node) = parse_stack.pop() {
        if let Ok(token) = Token::try_from(&node) {
            token_list.push(token);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(child) = node.child(i as u32) {
                parse_stack.push(child);
            }
        }
    }
    Some(token_list)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading Gemtext parser");
    }
}
