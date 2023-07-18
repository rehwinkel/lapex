#pragma once

#include <istream>
#include <cstdint>

namespace lexer
\{
    enum class TokenType : uint32_t
    \{
        TK_ERR = 0,
        TK_EOF = 1,
        {token_enum_variants}
    };

    const char *get_token_name(TokenType tk_type);

    class Lexer
    \{
        std::istream &in_chars;
        uint32_t ch;
        int err;
        size_t position;
        size_t start_pos;
        size_t end_pos;

    public:
        Lexer(std::istream &in_chars);
        TokenType next();
        size_t start();
        size_t end();
    };
}