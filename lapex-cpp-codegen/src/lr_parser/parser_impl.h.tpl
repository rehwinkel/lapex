#pragma once

#include "parser.h"
#include <queue>
#include <vector>

#include <iostream>

namespace parser
{
    enum class SymbolKind : uint8_t
    {
        Terminal,
        NonTerminal,
        State
    };

    struct Symbol
    {
        SymbolKind kind;
        uint32_t identifier;
    };

    enum class NonTerminalType : uint32_t
    {
        /*{non_terminal_enum_variants}*/
    };

    enum class ActionType : uint8_t
    {
        Shift,
        Reduce,
    };

    struct Action
    {
        ActionType action_type;
        uint16_t reduced_rule;
    };

    struct Transition
    {
        uint32_t next_state;
        bool is_accepting;
    };

    Action determine_action(uint32_t state, const lexer::TokenType &lookahead_tpken);

    void reduce_stack(uint32_t rule, std::vector<Symbol> &parse_stack, std::vector<Symbol> &rev_reduced_symbols);

    Transition retrieve_next_state(uint32_t state, const Symbol &current_symbol);

    template <class T>
    void reduce_visitor(Visitor<T> &visitor, const std::vector<Symbol> &rev_reduced_symbols, uint32_t rule)
    {
        /*{visitor_reduce_switch}*/
    }

    template <class T>
    Parser<T>::Parser(std::function<Token<T>()> token_function, Visitor<T> &visitor) : token_function(token_function), visitor(visitor) {}

    template <class T>
    void Parser<T>::parse()
    {
        std::queue<std::pair<lexer::TokenType, T>> lookahead;
        lookahead.push(this->token_function());

        std::vector<Symbol> parse_stack;
        Symbol entry_symbol{SymbolKind::State, static_cast<uint32_t>(/*{entry_state}*/)};
        parse_stack.push_back(entry_symbol);

        while (parse_stack.size() > 0)
        {
            auto next_token_and_data = lookahead.front();
            lexer::TokenType next_tk = next_token_and_data.first;
            Symbol next_symbol{SymbolKind::Terminal, static_cast<uint32_t>(next_tk)};

            uint32_t state = parse_stack.back().identifier;
            Action action = determine_action(state, next_tk);
            if (action.action_type == ActionType::Shift)
            {
                parse_stack.push_back(next_symbol);

                this->visitor.shift(next_tk, next_token_and_data.second);
                lookahead.pop();
                lookahead.push(this->token_function());
            }
            else if (action.action_type == ActionType::Reduce)
            {
                std::vector<Symbol> rev_reduced_symbols;
                reduce_stack(action.reduced_rule, parse_stack, rev_reduced_symbols);
                reduce_visitor(this->visitor, rev_reduced_symbols, action.reduced_rule);
            }
            Symbol current_symbol = parse_stack.back();
            uint32_t stack_state = parse_stack.at(parse_stack.size() - 2).identifier;
            Transition transition = retrieve_next_state(stack_state, current_symbol);
            if (transition.is_accepting)
            {
                parse_stack.pop_back();
                parse_stack.pop_back();
            }
            else
            {
                Symbol next_state_symbol{SymbolKind::State, transition.next_state};
                parse_stack.push_back(next_state_symbol);
            }
        }
    }
}
