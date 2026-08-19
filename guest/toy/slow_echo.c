#include "raddy.h"

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    static const char a[] = "A";
    static const char b[] = "B";
    raddy_resp_head((int32_t)(uintptr_t)RESP_HEAD, (int32_t)(sizeof(RESP_HEAD) - 1));
    raddy_resp_write((int32_t)(uintptr_t)a, 1);
    for (volatile uint32_t i = 0; i < 20000000u; i++) {
    }
    raddy_resp_write((int32_t)(uintptr_t)b, 1);
    raddy_resp_end();
    return 0;
}
