#include "raddy.h"

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    raddy_resp_head(0, 2147483647);
    return 0;
}
