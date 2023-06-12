#pragma once

#include "lexer.h"
#include <functional>
#include <utility>
#include <stack>
#include <queue>

#include <iostream>

namespace parser {

    /*INSERT_VISITOR*/

    template <class T>
    using TokenFunction = std::function<std::pair<lexer::TokenType, T>()>;

    struct Symbol {
        bool is_terminal;
        bool is_nt_exit;
        uint32_t identifier;
    };

    void push_production_from_table(Symbol non_terminal, lexer::TokenType lookahead, std::stack<Symbol>& parse_stack);

    void throw_unexpected_token_error(lexer::TokenType expected, lexer::TokenType got);

    template <class T>
    class Parser {
        TokenFunction<T> tokens;
        Visitor<T>& visitor;

        public:
            Parser(TokenFunction<T> tokens, Visitor<T>& visitor) : tokens(tokens), visitor(visitor) {}

            void exit_visitor(uint32_t non_terminal) {
                /*EXIT_SWITCH*/
            }

            void enter_visitor(uint32_t non_terminal) {
                /*ENTER_SWITCH*/
            }

            void parse() {
                std::queue<std::pair<lexer::TokenType, T>> lookahead;
                lookahead.push(this->tokens());

                std::stack<Symbol> parse_stack;
                Symbol end {true, false, static_cast<uint32_t>(lexer::TokenType::TK_EOF)};
                parse_stack.push(end);
                Symbol entry {false, false, /*INSERT_ENTRY*/};
                parse_stack.push(entry);

                while (parse_stack.size() > 0) {
                    Symbol current = parse_stack.top();
                    parse_stack.pop();
                    auto lookahead_token_and_data = lookahead.front();
                    lexer::TokenType lookahead_tk = lookahead_token_and_data.first;
                    if(current.is_nt_exit) {
                        this->exit_visitor(current.identifier);
                    } else if (!current.is_terminal) {
                        Symbol nt_exit_symbol{false, true, current.identifier};
                        parse_stack.push(nt_exit_symbol);
                        push_production_from_table(current, lookahead_tk, parse_stack);
                        this->enter_visitor(current.identifier);
                    } else {
                        if (current.identifier != static_cast<uint32_t>(lookahead_tk)) {
                            throw_unexpected_token_error(static_cast<lexer::TokenType>(current.identifier), lookahead_tk);
                        }
                        this->visitor.token(lookahead_tk, lookahead_token_and_data.second);
                        lookahead.pop();
                        lookahead.push(this->tokens());
                    }
                }
            }
    };

}
