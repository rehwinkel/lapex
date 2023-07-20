#pragma once

#include "tokens.h"
#include <istream>
#include <cstdint>

namespace lexer
{
    class Lexer
    {
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