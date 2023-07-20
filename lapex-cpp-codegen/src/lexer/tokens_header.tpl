#pragma once

#include <cstdint>

namespace lexer
{
    enum class TokenType : uint32_t
    {
        TK_ERR = 0,
        TK_EOF = 1,
        /*{token_enum_variants}*/
    };
    
    const char *get_token_name(TokenType tk_type);
}