#pragma once

#include "tokens.h"
#include "visitor.h"
#include <functional>
#include <utility>

namespace parser
{

    template <class T>
    using Token = std::pair<lexer::TokenType, T>;

    template <class T>
    class Parser
    {
    private:
        std::function<Token<T>()> token_function;
        Visitor<T> &visitor;

    public:
        Parser(std::function<Token<T>()> token_function, Visitor<T> &visitor);

        void parse();
    };

}
