#include "parser_impl.h"

#include <sstream>

namespace parser
{
    void throw_unexpected_token_error(const char* expected, lexer::TokenType got) {
        std::ostringstream os;
        os << "Unexpected token '" << lexer::get_token_name(got) << "', expected one of: " << expected;
        throw std::runtime_error(os.str());
    }

    Action determine_action(uint32_t state, const lexer::TokenType &lookahead_token)
    {
        /*{action_table}*/
    }

    void reduce_stack(uint32_t rule, std::vector<Symbol> &parse_stack, std::vector<Symbol> &rev_reduced_symbols)
    {
        /*{stack_reduce_table}*/
    }

    Transition retrieve_next_state(uint32_t state, const Symbol &current_symbol)
    {
        /*{goto_table}*/
    }
}