#include "raddy.h"

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    raddy_resp_head((int32_t)(uintptr_t)RESP_HEAD, (int32_t)(sizeof(RESP_HEAD) - 1));
    raddy_resp_write(1000000, 8);
    raddy_resp_end();
    return 0;
}
