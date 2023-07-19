#pragma once

#include "lexer.h"

namespace parser
\{
    template <class T>
    class Visitor
    \{
    public:
        virtual void token(lexer::TokenType tk_type, T data) = 0;
        {visitor_methods}
    };
}