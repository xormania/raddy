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

static uint8_t heap[131072];

static const char RESP_HEAD[] =
    "{\"v\":1,\"status\":200,\"headers\":[[\"content-type\",\"application/octet-stream\"]]}";

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    int32_t hlen = raddy_head_len();
    if (hlen < 0 || hlen > 32768) {
        return 1;
    }
    int32_t n = raddy_head_read((int32_t)(uintptr_t)heap, hlen);
    if (n != hlen) {
        return 1;
    }

    int32_t body_off = hlen;
    for (;;) {
        int32_t space = (int32_t)sizeof(heap) - body_off;
        if (space <= 0) {
            return 1;
        }
        int32_t got = raddy_body_read((int32_t)(uintptr_t)(heap + body_off), space);
        if (got < 0) {
            return 1;
        }
        if (got == 0) {
            break;
        }
        body_off += got;
    }

    int32_t body_len = body_off - hlen;
    if (raddy_resp_head((int32_t)(uintptr_t)RESP_HEAD, (int32_t)(sizeof(RESP_HEAD) - 1)) < 0) {
        return 1;
    }
    if (raddy_resp_write((int32_t)(uintptr_t)heap, hlen) < 0) {
        return 1;
    }
    if (body_len > 0
        && raddy_resp_write((int32_t)(uintptr_t)(heap + hlen), body_len) < 0)
    {
        return 1;
    }
    if (raddy_resp_end() < 0) {
        return 1;
    }
    return 0;
}
