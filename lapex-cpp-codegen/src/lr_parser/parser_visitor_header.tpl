#pragma once

#include "tokens.h"

namespace parser
\{
    template <class T>
    class Visitor
    \{
    public:
        virtual void shift(lexer::TokenType tk_type, T data) = 0;
        {visitor_methods}
    };
}