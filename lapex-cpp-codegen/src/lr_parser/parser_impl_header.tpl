#pragma once

#include "parser.h"
#include <queue>
#include <vector>

#include <iostream>

namespace parser
\{
    enum class SymbolKind : uint8_t
    \{
        Terminal,
        NonTerminal,
        State
    };

    struct Symbol
    \{
        SymbolKind kind;
        uint32_t identifier;
    };
    
    enum class NonTerminalType : uint32_t
    \{
        {non_terminal_enum_variants}
    };

    uint32_t determine_action(uint32_t state, const Symbol& lookahead_symbol);

    bool reduce_stack(uint32_t rule, std::vector<Symbol> &parse_stack, std::vector<Symbol>& rev_reduced_symbols);

    uint32_t retrieve_next_state(uint32_t state, const Symbol& current_symbol);

    template <class T>
    void reduce_visitor(Visitor<T>& visitor, const std::vector<Symbol> &rev_reduced_symbols, uint32_t rule)
    \{
        {visitor_reduce_switch}
    }

    template <class T>
    Parser<T>::Parser(std::function<Token<T>()> token_function, Visitor<T> &visitor) : token_function(token_function), visitor(visitor) \{}

    template <class T>
    void Parser<T>::parse()
    \{
        std::queue<std::pair<lexer::TokenType, T>> lookahead;
        lookahead.push(this->token_function());

        std::vector<Symbol> parse_stack;
        Symbol entry_symbol\{SymbolKind::State, static_cast<uint32_t>({entry_state})};
        parse_stack.push_back(entry_symbol);

        while (parse_stack.size() > 0)
        \{
            auto next_token_and_data = lookahead.front();
            lexer::TokenType next_tk = next_token_and_data.first;  
            Symbol next_symbol\{SymbolKind::Terminal, static_cast<uint32_t>(next_tk)};
            
            uint32_t state = parse_stack.back().identifier;
            uint32_t action = determine_action(state, next_symbol);
            if (action == 0) \{
                parse_stack.push_back(next_symbol);

                this->visitor.shift(next_tk, next_token_and_data.second);
                lookahead.pop();
                lookahead.push(this->token_function());
            } else \{
                std::vector<Symbol> rev_reduced_symbols;
                bool accepted = reduce_stack(action, parse_stack, rev_reduced_symbols);
                reduce_visitor(this->visitor, rev_reduced_symbols, action);
                if (accepted) \{
                    continue;
                }
            }
            Symbol current_symbol = parse_stack.back();
            uint32_t stack_state = parse_stack.at(parse_stack.size() - 2).identifier;
            uint32_t next_state = retrieve_next_state(stack_state, current_symbol);
            Symbol next_state_symbol\{SymbolKind::State, next_state};
            parse_stack.push_back(next_state_symbol);
        }
    }
}
