// Copyright (c) 2026 tree-sitter-gemtext contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#include <assert.h>
#include <stdbool.h>
#include <stddef.h> // For size_t
#include <string.h> // For memcpy & memset

#include "tree_sitter/alloc.h"
#include "tree_sitter/parser.h"

// These must match the order in `externals` array in grammar.js
enum Token {
    TEXT = 0,
    BLOCKQUOTE,
    BLOCKQUOTE_TEXT,
    HEADING,
    HEADING_TEXT,
    LINK,
    LINK_URL,
    LINK_LABEL,
    LIST,
    LIST_TEXT,
    PREFORMATTED_BEGIN,
    PREFORMATTED_TEXT,
    PREFORMATTED_END,
};

typedef struct {
    enum Token last;
    TSLexer *l;
} State;

#define STATE_SIZE sizeof(State)

// --- Standard Tree-sitter Scanner Functions ---

void *tree_sitter_gemtext_external_scanner_create() {
    State *s = (State *)ts_malloc(STATE_SIZE);
    memset(s, 0, STATE_SIZE);
    return s;
}

void tree_sitter_gemtext_external_scanner_destroy(void *payload) {
    State *s = (State *)payload;
    ts_free(s);
}

unsigned tree_sitter_gemtext_external_scanner_serialize(void *payload,
                                                        char *buffer) {
    memcpy(buffer, payload, STATE_SIZE);
    return STATE_SIZE;
}

void tree_sitter_gemtext_external_scanner_deserialize(void *payload,
                                                      const char *buffer,
                                                      unsigned length) {
    if (length == STATE_SIZE) {
        memcpy(payload, buffer, STATE_SIZE);
    } else {
        memset(payload, 0, STATE_SIZE);
    }
}

// --- Helper
static inline int32_t lookahead(State *s) { return s->l->lookahead; }

static inline uint32_t column(State *s) { return s->l->get_column(s->l); }

static inline bool eof(State *s) { return s->l->eof(s->l); }

static inline void markend(State *s) { return s->l->mark_end(s->l); }

// Consume one character (included in the token range).
static inline void consume(State *s) { s->l->advance(s->l, false); }

// Advance until `c` is found (not consumed). Returns false on EOF.
static bool skip_until(State *s, int32_t c) {
    while (1) {
        if (eof(s)) {
            return false;
        }
        if (lookahead(s) == c) {
            return true;
        }
        consume(s);
    }
}

static inline void emit(State *s, TSSymbol token) {
    s->l->result_symbol = token;
    s->last = token;
}

static bool emit_text(State *s) {
    // emit one-line text
    if (!skip_until(s, '\n')) {
        emit(s, TEXT);
        return true;
    }
    emit(s, TEXT);
    consume(s); // consume \n
    return true;
}

static bool emit_blockquote(State *s) {
    consume(s);
    emit(s, BLOCKQUOTE);
    return true;
}

static bool emit_blockquote_text(State *s) {
    if (!skip_until(s, '\n')) {
        emit(s, BLOCKQUOTE_TEXT);
        return true;
    }
    emit(s, BLOCKQUOTE_TEXT);
    consume(s);
    return true;
}

// "# ..." / "## ..." / "### ..."
// The space after the #'s is mandatory per spec.
// "#text" (no space) is treated as TEXT.
static bool emit_heading(State *s) {
    // Consume up to 3 '#' characters.
    size_t count = 0;
    while (count < 3 && !eof(s) && lookahead(s) == '#') {
        consume(s);
        count++;
    }

    // Must have a space right after the #'s to be a valid heading.
    if (count >= 1 && !eof(s) && lookahead(s) == ' ') {
        emit(s, HEADING);
        return true;
    }

    // Not a valid heading — treat the whole line as TEXT.
    return emit_text(s);
}

static bool emit_heading_text(State *s) {
    if (!skip_until(s, '\n')) {
        emit(s, HEADING_TEXT);
        return true;
    }
    emit(s, HEADING_TEXT);
    consume(s);
    return true;
}

// "=> URL[ label]"
static bool emit_link(State *s) {
    consume(s); // consume '='

    if (!eof(s) && lookahead(s) == '>') {
        consume(s); // consume '>'
        emit(s, LINK);
        return true;
    }

    // Not a valid link — treat the rest as TEXT.
    return emit_text(s);
}

static bool emit_link_url(State *s) {
    // Skip leading whitespace (included in the token range).
    while (!eof(s) && lookahead(s) == ' ') {
        consume(s);
    }
    // Consume URL content until space, newline, or EOF.
    // The delimiter space (between URL and label) is included in the URL token.
    while (!eof(s)) {
        int32_t c = lookahead(s);
        if (c == ' ' || c == '\n') {
            if (c == ' ') {
                consume(s); // include trailing space in URL token
            }
            markend(s);
            emit(s, LINK_URL);
            return true;
        }
        consume(s);
    }
    markend(s);
    emit(s, LINK_URL);
    return true;
}

static bool emit_link_label(State *s) {
    if (!skip_until(s, '\n')) {
        emit(s, LINK_LABEL);
        return true;
    }
    emit(s, LINK_LABEL);
    consume(s);
    return true;
}

// "* " (mandatory space after *)
static bool emit_list(State *s) {
    consume(s); // consume '*'

    if (!eof(s) && lookahead(s) == ' ') {
        emit(s, LIST);
        return true;
    }

    // Not a valid list item — treat as TEXT.
    return emit_text(s);
}

static bool emit_list_text(State *s) {
    if (!skip_until(s, '\n')) {
        emit(s, LIST_TEXT);
        return true;
    }
    emit(s, LIST_TEXT);
    consume(s); // consume '\n'
    return true;
}

// ```...```
// Parse a preformatted block into three external tokens: begin, text, and end.
static bool emit_preformatted_begin(State *s) {
    // Try to read 3 backticks.  If we get fewer, treat the line as TEXT.
    for (int i = 0; i < 3; i++) {
        if (eof(s) || lookahead(s) != '`') {
            // Partial match: the backticks consumed so far are part of the
            // TEXT token. Finish consuming the rest of the line as TEXT.
            return emit_text(s);
        }
        consume(s);
    }
    emit(s, PREFORMATTED_BEGIN);
    return true;
}

static bool emit_preformatted_text(State *s) {
    // We have consumed the opening ```.  Now collect all preformatted content
    // until the closing ``` or EOF. The closing marker itself is left for
    // emit_preformatted_end().
    while (1) {
        if (!skip_until(s, '`')) {
            // Unclosed preformatted block — emit what we have.
            markend(s);
            emit(s, PREFORMATTED_TEXT);
            return true;
        }

        // Candidate closing marker begins here.
        if (!eof(s) && lookahead(s) == '`') {
            // Mark the end of the preformatted content before the potential
            // closing marker.
            markend(s);
            consume(s);
            if (!eof(s) && lookahead(s) == '`') {
                consume(s);
                if (!eof(s) && lookahead(s) == '`') {
                    // Closing marker confirmed; do not consume the third
                    // backtick.
                    emit(s, PREFORMATTED_TEXT);
                    return true;
                }
            }
        }

        // Not a closing marker; continue scanning and let the preformatted text
        // include the characters consumed while probing.
    }
}

static bool emit_preformatted_end(State *s) {
    // Consume the closing ``` marker and emit it as its own token.
    for (int i = 0; i < 3; i++) {
        if (eof(s) || lookahead(s) != '`') {
            break;
        }
        consume(s);
    }
    if (!eof(s) && lookahead(s) == '\n') {
        consume(s);
    }
    markend(s);
    emit(s, PREFORMATTED_END);
    return true;
}

static bool parse_line(State *s) {
    if (eof(s)) {
        return false;
    }
    if (column(s) != 0) {
        // If the current position is not at the start of a line, fall back to
        // plain text parsing.
        return emit_text(s);
    }
    switch (lookahead(s)) {
    case '>':
        return emit_blockquote(s);
    case '#':
        return emit_heading(s);
    case '=':
        return emit_link(s);
    case '*':
        return emit_list(s);
    case '`':
        return emit_preformatted_begin(s);
    default:
        break;
    }
    return emit_text(s);
}

// --- Main dispatch ---

static bool emit_token(State *s) {
    switch (s->last) {
    case BLOCKQUOTE_TEXT:
    case HEADING_TEXT:
    case LIST_TEXT:
    case LINK_LABEL:
    case TEXT:
        return parse_line(s);
    case BLOCKQUOTE:
        return emit_blockquote_text(s);
    case HEADING:
        return emit_heading_text(s);
    case LINK:
        return emit_link_url(s);
    case LINK_URL:
        return emit_link_label(s);
    case LIST:
        return emit_list_text(s);
    case PREFORMATTED_BEGIN:
        return emit_preformatted_text(s);
    case PREFORMATTED_TEXT:
        return emit_preformatted_end(s);
    case PREFORMATTED_END:
        return parse_line(s);
    default:
        break;
    }
    // If no valid parser transition exists, only attempt a final text token at EOF.
    return eof(s) ? emit_text(s) : false;
}

// Main
bool tree_sitter_gemtext_external_scanner_scan(void *payload, TSLexer *l,
                                               const bool *valid_symbols) {
    (void)valid_symbols;
    State *s = (State *)payload;
    s->l = l;
    return emit_token(s);
}
