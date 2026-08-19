#include "raddy.h"

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    static const char bad[] = "x";
    raddy_resp_head((int32_t)(uintptr_t)bad, 1);
    return 0;
}
