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

use std::{ops::Range, str::FromStr};

mod token_types;

pub use token_types::TokenType;

/// A value associated with a byte-span in the source text.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T>(pub T, pub Range<usize>);

/// A parsed token with its source range.
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
        // Convert only recognized parser node kinds into tokens.
        if let Ok(t) = TokenType::from_str(node.kind()) {
            let r = node.range();
            return Ok(Token::new(t, r.start_byte..r.end_byte));
        }
        Err(())
    }
}

/// Parse the source and return the stream of recognized tokens in document order.
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
        // Push children in reverse so we visit nodes in source order.
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
    use crate::{
        Token,
        TokenType::{
            Blockquote, BlockquoteText, Heading, HeadingText, Link, LinkLabel, LinkUrl, List,
            ListText, PreformattedBegin, PreformattedEnd, PreformattedText, Text,
        },
        tokenize,
    };

    macro_rules! must_match {
        ($case:expr, $expects:expr) => {{
            let tokens = tokenize($case);
            assert!(tokens.is_some());
            let tokens = tokens.unwrap();
            assert_eq!(tokens.len(), $expects.len(), "tokens: {:?}", tokens);
            for (id, expect) in $expects.iter().enumerate() {
                if &tokens[id] != expect {
                    panic!(
                        "token mismatch at {}: got {:?}, expected {:?}\ncase: {:?}",
                        id, tokens[id], expect, $case,
                    );
                }
            }
        }};
    }

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading Gemtext parser");
    }

    #[test]
    fn test_can_parse_simple_gemtext() {
        let source = r#">
# Hello
=> https://example.com Example
* item
```
code block
```
Text line
"#;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading Gemtext parser");
        let tree = parser.parse(source, None).expect("Failed to parse source");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_can_tokenize_blockquote() {
        must_match!(
            ">",
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..1),
            ]
        );

        must_match!(
            "> ",
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..2),
            ]
        );

        must_match!(
            ">a",
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..2),
            ]
        );

        must_match!(
            r#">a
bob"#,
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..3),
                Token::new(Text, 3..6)
            ]
        );

        must_match!(
            ">hello world",
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..12),
            ]
        );
    }

    #[test]
    fn test_can_tokenize_heading() {
        must_match!(
            "# Hello",
            &[Token::new(Heading, 0..1), Token::new(HeadingText, 1..7),]
        );
    }

    #[test]
    fn test_can_tokenize_link() {
        must_match!(
            "=> https://example.com Example",
            &[
                Token::new(Link, 0..2),
                Token::new(LinkUrl, 2..23),
                Token::new(LinkLabel, 23..30),
            ]
        );
    }

    #[test]
    fn test_can_tokenize_list() {
        must_match!(
            "* item",
            &[Token::new(List, 0..1), Token::new(ListText, 1..6),]
        );
    }

    #[test]
    fn test_can_tokenize_preformatted() {
        must_match!(
            "```
code block
```",
            &[
                Token::new(PreformattedBegin, 0..3),
                Token::new(PreformattedText, 3..4),
                Token::new(PreformattedText, 4..15),
                Token::new(PreformattedEnd, 15..18),
            ]
        );
    }

    #[test]
    fn test_can_tokenize_text() {
        must_match!("Text line", &[Token::new(Text, 0..9),]);
    }

    #[test]
    fn test_can_tokenize_full_mixed() {
        must_match!(
            "> Quote\n# Heading\n=> https://example.com Label\n* item\n```\ncode\n```\nPlain text",
            &[
                Token::new(Blockquote, 0..1),
                Token::new(BlockquoteText, 1..8),
                Token::new(Heading, 8..9),
                Token::new(HeadingText, 9..18),
                Token::new(Link, 18..20),
                Token::new(LinkUrl, 20..41),
                Token::new(LinkLabel, 41..47),
                Token::new(List, 47..48),
                Token::new(ListText, 48..54),
                Token::new(PreformattedBegin, 54..57),
                Token::new(PreformattedText, 57..58),
                Token::new(PreformattedText, 58..63),
                Token::new(PreformattedEnd, 63..67),
                Token::new(Text, 67..77),
            ]
        );
    }
}
