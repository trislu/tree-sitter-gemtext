use strum_macros::{Display, EnumCount, EnumIter, EnumString};

#[derive(Clone, Copy, Debug, Display, EnumCount, EnumIter, EnumString, Eq, Hash, PartialEq)]
pub enum TokenType {
    #[strum( serialize = "blockquote")]
    Blockquote,
    #[strum( serialize = "blockquote_text")]
    BlockquoteText,
    #[strum( serialize = "heading")]
    Heading,
    #[strum( serialize = "heading_text")]
    HeadingText,
    #[strum( serialize = "link")]
    Link,
    #[strum( serialize = "link_label")]
    LinkLabel,
    #[strum( serialize = "link_url")]
    LinkUrl,
    #[strum( serialize = "list")]
    List,
    #[strum( serialize = "list_text")]
    ListText,
    #[strum( serialize = "preformatted_begin")]
    PreformattedBegin,
    #[strum( serialize = "preformatted_end")]
    PreformattedEnd,
    #[strum( serialize = "preformatted_text")]
    PreformattedText,
    #[strum( serialize = "text")]
    Text,
}