#include "parser.h"
#include <sstream>

struct TokenData {
  size_t start;
  size_t end;
};

class MyVisitor : public parser::Visitor<TokenData> {
public:
  virtual void enter_sum() {}
  virtual void exit_sum() {}
  virtual void enter_factor() {}
  virtual void exit_factor() {}
  virtual void enter_operand() {}
  virtual void exit_operand() {}

  virtual void enter_Session() {}
  virtual void exit_Session() {}
  virtual void enter_Facts() {}
  virtual void exit_Facts() {}
  virtual void enter_Question() {}
  virtual void exit_Question() {}
  virtual void enter_Fact() {}
  virtual void exit_Fact() {}
  virtual void token(lexer::TokenType tk_type, TokenData data) {}
};

int main() {
  std::stringstream ss;
  std::string contents = "!string!string?string";
  ss.write(contents.c_str(), contents.size());
  lexer::Lexer l(ss);
  MyVisitor vis;
  parser::Parser<TokenData> p(
      [&l]() {
        lexer::TokenType tk = l.next();
        TokenData data{l.start(), l.end()};
        return std::make_pair(tk, data);
      },
      vis);
  p.parse();
  return 0;
}