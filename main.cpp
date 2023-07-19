#include "parser.h"
#include <sstream>
#include <iostream>

struct TokenData {
  size_t start;
  size_t end;
};

class MyVisitor : public parser::Visitor<TokenData> {
private:
  uint32_t current_indent;
  std::ostream &indent(std::ostream &stream) {
    if (current_indent > 100)
      throw std::runtime_error("Indent too large");
    for (uint32_t i = 0; i < current_indent; i++) {
      stream << "    ";
    }
    return stream;
  }

public:
  MyVisitor() : current_indent(0) {}
  virtual void enter_sum() {}
  virtual void exit_sum() {}
  virtual void enter_factor() {}
  virtual void exit_factor() {}
  virtual void enter_operand() {}
  virtual void exit_operand() {}

  virtual void enter_Session() {
    this->indent(std::cout) << "enter session" << std::endl;
    this->current_indent++;
  }
  virtual void exit_Session() {
    this->current_indent--;
    this->indent(std::cout) << "exit session" << std::endl;
  }
  virtual void enter_Facts() {
    this->indent(std::cout) << "enter facts" << std::endl;
    this->current_indent++;
  }
  virtual void exit_Facts() {
    this->current_indent--;
    this->indent(std::cout) << "exit facts" << std::endl;
  }
  virtual void enter_Question() {
    this->indent(std::cout) << "enter question" << std::endl;
    this->current_indent++;
  }
  virtual void exit_Question() {
    this->current_indent--;
    this->indent(std::cout) << "exit question" << std::endl;
  }
  virtual void enter_Fact() {
    this->indent(std::cout) << "enter fact" << std::endl;
    this->current_indent++;
  }
  virtual void exit_Fact() {
    this->current_indent--;
    this->indent(std::cout) << "exit fact" << std::endl;
  }
  virtual void token(lexer::TokenType tk_type, TokenData data) {
    this->indent(std::cout)
        << "Token " << lexer::get_token_name(tk_type) << std::endl;
  }
};

int main() {
  lexer::Lexer l(std::cin);
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