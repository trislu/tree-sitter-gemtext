/**
 * @file A parser for gemtext
 * @author room2 <lukai@mail.ustc.edu.cn>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "gemtext",

  rules: {
    // TODO: add the actual grammar rules
    source_file: $ => "hello"
  }
});
