#include "raddy.h"

static uint8_t a[1];
static uint8_t b[1];

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    if (raddy_body_read((int32_t)(uintptr_t)a, 1) != 1) {
        return 1;
    }
    if (raddy_body_read((int32_t)(uintptr_t)b, 1) != 1) {
        return 1;
    }
    raddy_resp_head((int32_t)(uintptr_t)RESP_HEAD, (int32_t)(sizeof(RESP_HEAD) - 1));
    raddy_resp_write((int32_t)(uintptr_t)a, 1);
    raddy_resp_write((int32_t)(uintptr_t)b, 1);
    raddy_resp_end();
    return 0;
}
