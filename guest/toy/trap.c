#include <stdint.h>

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    __builtin_trap();
    return 1;
}
