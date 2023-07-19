#include "parser_impl.h"

#include <sstream>

namespace parser \{
    void push_production_from_table(Symbol non_terminal, lexer::TokenType lookahead, std::stack<Symbol>& parse_stack) \{
        {parser_table_switch}
    }


    void throw_unexpected_token_error(lexer::TokenType expected, lexer::TokenType got) \{
        std::ostringstream os;
        os << "Unexpected token '" << lexer::get_token_name(got) << "', expected token '" << lexer::get_token_name(expected) << "'";
        throw std::runtime_error(os.str());
    }
}