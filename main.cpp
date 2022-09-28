#include "lexer.h"
#include <sstream>
#include <iostream>

int main(int argc, char const *argv[])
{
    std::stringstream ss;
    ss.write("3 * 13 + 4 / 52 - 11 + 87", 25);
    lexer::Lexer l(ss);
    while (1)
    {
        lexer::TokenType tk = l.next();
        std::cout << int(tk) << " (" << l.start() << " - " << l.end() << ")" << std::endl;
        if (tk == lexer::TokenType::TK_EOF || tk == lexer::TokenType::TK_ERR)
        {
            break;
        }
    }
    return 0;
}
