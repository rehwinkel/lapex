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

            void parse() {
                std::queue<std::pair<lexer::TokenType, T>> lookahead;
                lookahead.push(this->tokens());

                std::stack<Symbol> parse_stack;
                Symbol end {true, static_cast<uint32_t>(lexer::TokenType::TK_EOF)};
                parse_stack.push(end);
                Symbol entry {false, /*INSERT_ENTRY*/};
                parse_stack.push(entry);

                while (parse_stack.size() > 0) {
                    Symbol current = parse_stack.top();
                    parse_stack.pop();
                    auto lookahead_token_and_data = lookahead.front();
                    lexer::TokenType lookahead_tk = lookahead_token_and_data.first;
                    if (current.is_terminal) {
                        std::cout << "on stack: TK " << lexer::get_token_name(static_cast<lexer::TokenType>(current.identifier)) << std::endl;
                    } else {
                        std::cout << "on stack: NT" << current.identifier << std::endl;
                    }
                    std::cout << "lookahead: " << lexer::get_token_name(lookahead_tk) << std::endl;
                    if (!current.is_terminal) {
                        push_production_from_table(current, lookahead_tk, parse_stack);
                    } else {
                        if (current.identifier != static_cast<uint32_t>(lookahead_tk)) {
                            throw_unexpected_token_error(static_cast<lexer::TokenType>(current.identifier), lookahead_tk);
                        }
                        lookahead.pop();
                        lookahead.push(this->tokens());
                    }
                }
            }
    };

}
