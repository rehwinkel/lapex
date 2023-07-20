#include "parser_impl.h"

#include <sstream>

namespace parser {
    uint32_t determine_action(uint32_t state, const Symbol& lookahead_symbol) {
        /*{action_table}*/
    }

    bool reduce_stack(uint32_t rule, std::vector<Symbol> &parse_stack, std::vector<Symbol>& rev_reduced_symbols) {
        /*{stack_reduce_table}*/
    }

    uint32_t retrieve_next_state(uint32_t state, const Symbol& current_symbol) {
        /*{goto_table}*/
    }
}