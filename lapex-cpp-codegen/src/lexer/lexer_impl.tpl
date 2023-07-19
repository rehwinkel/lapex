#include "lexer.h"

namespace lexer
\{
    Lexer::Lexer(std::istream &in) : in_chars(in), ch(-1), err(0), start_pos(0), end_pos(0), position(0) \{}

    // Branchless UTF-8: https://github.com/skeeto/branchless-utf8
    void utf8_decode(std::istream &in, uint32_t *c, int *e)
    \{
        static const char lengths[] = \{
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 3, 3, 4, 0};
        static const int masks[] = \{0x00, 0x7f, 0x1f, 0x0f, 0x07};
        static const uint32_t mins[] = \{4194304, 0, 128, 2048, 65536};
        static const int shiftc[] = \{0, 18, 12, 6, 0};
        static const int shifte[] = \{0, 6, 4, 2, 0};

        char buf[4] = \{0};
        in.read(buf, 1);
        uint8_t *s = (uint8_t *)buf;
        int len = lengths[s[0] >> 3];
        in.read(buf + 1, (len > 1) * (len - 1));

        /* Assume a four-byte character and load four bytes. Unused bits are
         * shifted out.
         */
        *c = (uint32_t)(s[0] & masks[len]) << 18;
        *c |= (uint32_t)(s[1] & 0x3f) << 12;
        *c |= (uint32_t)(s[2] & 0x3f) << 6;
        *c |= (uint32_t)(s[3] & 0x3f) << 0;
        *c >>= shiftc[len];

        /* Accumulate the various error conditions. */
        *e = (*c < mins[len]) << 6;      // non-canonical encoding
        *e |= ((*c >> 11) == 0x1b) << 7; // surrogate half?
        *e |= (*c > 0x10FFFF) << 8;      // out of range?
        *e |= (s[1] & 0xc0) >> 2;
        *e |= (s[2] & 0xc0) >> 4;
        *e |= (s[3]) >> 6;
        *e ^= 0x2a; // top two bits of each tail byte correct?
        *e >>= shifte[len];
    }

    size_t Lexer::start()
    \{
        return this->start_pos;
    }
    size_t Lexer::end()
    \{
        return this->end_pos;
    }

    TokenType Lexer::next()
    \{
        uint32_t state = 0;
        this->start_pos = position;
        while (1)
        \{
            if (this->ch == -1)
            \{
                utf8_decode(this->in_chars, &this->ch, &this->err);
            }
            if (this->err)
            \{
                return TokenType::TK_ERR;
            }

            {alphabet_switch}
            {automaton_switch}
            this->position += 1;
        }
        return TokenType::TK_ERR;
    }
}