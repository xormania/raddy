#include "raddy.h"

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    static const char x[] = "x";
    raddy_resp_head((int32_t)(uintptr_t)RESP_HEAD, (int32_t)(sizeof(RESP_HEAD) - 1));
    for (int i = 0; i < 32; i++) {
        raddy_resp_write((int32_t)(uintptr_t)x, 1);
    }
    raddy_resp_end();
    return 0;
}
