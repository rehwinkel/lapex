#pragma once

#include "parser.h"
#include <stack>
#include <queue>

#include <iostream>

namespace parser
\{
    void push_production_from_table(Symbol non_terminal, lexer::TokenType lookahead, std::stack<Symbol> &parse_stack);

    void throw_unexpected_token_error(lexer::TokenType expected, lexer::TokenType got);
    
    enum class NonTerminalType : uint32_t
    \{
        {non_terminal_enum_variants}
    };
    
    template <class T>
    void exit_visitor(Visitor<T>& visitor, NonTerminalType non_terminal)
    \{
        {visitor_exit_switch}
    }

    template <class T>
    void enter_visitor(Visitor<T>& visitor, NonTerminalType non_terminal)
    \{
        {visitor_enter_switch}
    }

    template <class T>
    Parser<T>::Parser(std::function<Token<T>()> token_function, Visitor<T> &visitor) : token_function(token_function), visitor(visitor) \{}

    template <class T>
    void Parser<T>::parse()
    \{
        std::queue<std::pair<lexer::TokenType, T>> lookahead;
        lookahead.push(this->token_function());

        std::stack<Symbol> parse_stack;
        Symbol end\{SymbolKind::Terminal, static_cast<uint32_t>(lexer::TokenType::TK_EOF)};
        parse_stack.push(end);
        Symbol entry\{SymbolKind::NonTerminal, static_cast<uint32_t>({grammar_entry_non_terminal})};
        parse_stack.push(entry);

        while (parse_stack.size() > 0)
        \{
            Symbol current = parse_stack.top();
            parse_stack.pop();
            auto lookahead_token_and_data = lookahead.front();
            lexer::TokenType lookahead_tk = lookahead_token_and_data.first;
            if (current.kind == SymbolKind::ExitNonTerminal)
            \{
                exit_visitor(this->visitor, static_cast<NonTerminalType>(current.identifier));
            }
            else if (current.kind != SymbolKind::Terminal)
            \{
                Symbol nt_exit_symbol\{SymbolKind::ExitNonTerminal, current.identifier};
                parse_stack.push(nt_exit_symbol);
                push_production_from_table(current, lookahead_tk, parse_stack);
                enter_visitor(this->visitor, static_cast<NonTerminalType>(current.identifier));
            }
            else
            \{
                if (current.identifier != static_cast<uint32_t>(lookahead_tk))
                \{
                    throw_unexpected_token_error(static_cast<lexer::TokenType>(current.identifier), lookahead_tk);
                }
                this->visitor.token(lookahead_tk, lookahead_token_and_data.second);
                lookahead.pop();
                lookahead.push(this->token_function());
            }
        }
    }
}
