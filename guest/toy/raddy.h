#ifndef RADDY_TOY_H
#define RADDY_TOY_H

#include <stdint.h>

__attribute__((import_module("raddy"), import_name("raddy_head_len")))
int32_t raddy_head_len(void);
__attribute__((import_module("raddy"), import_name("raddy_head_read")))
int32_t raddy_head_read(int32_t ptr, int32_t cap);
__attribute__((import_module("raddy"), import_name("raddy_body_read")))
int32_t raddy_body_read(int32_t ptr, int32_t cap);
__attribute__((import_module("raddy"), import_name("raddy_resp_head")))
int32_t raddy_resp_head(int32_t ptr, int32_t len);
__attribute__((import_module("raddy"), import_name("raddy_resp_write")))
int32_t raddy_resp_write(int32_t ptr, int32_t len);
__attribute__((import_module("raddy"), import_name("raddy_resp_end")))
int32_t raddy_resp_end(void);

static const char RESP_HEAD[] =
    "{\"v\":1,\"status\":200,\"headers\":[[\"content-type\",\"application/octet-stream\"]]}";

#endif
