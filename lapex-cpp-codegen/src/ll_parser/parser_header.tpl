#pragma once

#include "lexer.h"
#include "visitor.h"
#include <functional>
#include <utility>

namespace parser
\{

    template <class T>
    using Token = std::pair<lexer::TokenType, T>;

    enum class SymbolKind : uint8_t
    \{
        Terminal,
        NonTerminal,
        ExitNonTerminal
    };

    struct Symbol
    \{
        SymbolKind kind;
        uint32_t identifier;
    };

    template <class T>
    class Parser
    \{
    private:
        std::function<Token<T>()> token_function;
        Visitor<T> &visitor;

    public:
        Parser(std::function<Token<T>()> token_function, Visitor<T> &visitor);

        void parse();
    };

}
