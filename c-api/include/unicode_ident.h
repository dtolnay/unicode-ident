#ifndef UNICODE_IDENT_H
#define UNICODE_IDENT_H

#include <stdbool.h>
#include <stdint.h>

bool is_xid_start(uint32_t ch);
bool is_xid_continue(uint32_t ch);

#endif
