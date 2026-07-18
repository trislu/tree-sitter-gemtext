/**
 * @file A parser for gemtext
 * @author room2 <lukai@mail.ustc.edu.cn>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "gemtext",

  externals: $ => [
    $.text,
    $.blockquote,
    $.blockquote_text,
    $.heading,
    $.heading_text,
    $.link,
    $.link_url,
    $.link_label,
    $.list,
    $.list_text,
    $.preformatted_begin,
    $.preformatted_text,
    $.preformatted_end,
  ],

  rules: {
    source_file: $ =>
      repeat(
        choice(
          seq($.blockquote, $.blockquote_text),
          seq($.heading, $.heading_text),
          seq($.link, $.link_url, $.link_label),
          seq($.list, $.list_text),
          seq($.preformatted_begin, repeat1($.preformatted_text), $.preformatted_end),
          $.text,
        ),
      ),
  },
})
