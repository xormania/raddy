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

__attribute__((import_module("wasi_snapshot_preview1"), import_name("random_get")))
int32_t wasi_random_get(int32_t ptr, int32_t len);
__attribute__((import_module("wasi_snapshot_preview1"), import_name("clock_time_get")))
int32_t wasi_clock_time_get(int32_t id, int64_t precision, int32_t out);

static uint8_t heap[65536];
static int booted;

__attribute__((export_name("wizer.initialize")))
void wizer_initialize(void) {
    booted = 1;
}

static int find(const uint8_t *s, int n, const char *needle) {
    int k = 0;
    while (needle[k] != 0) {
        k++;
    }
    if (k == 0 || k > n) {
        return -1;
    }
    for (int i = 0; i + k <= n; i++) {
        int ok = 1;
        for (int j = 0; j < k; j++) {
            if (s[i + j] != (uint8_t)needle[j]) {
                ok = 0;
                break;
            }
        }
        if (ok) {
            return i;
        }
    }
    return -1;
}

static int copy_until(const uint8_t *s, int n, int off, char end, char *out, int cap) {
    int i = 0;
    while (off + i < n && s[off + i] != (uint8_t)end && i + 1 < cap) {
        out[i] = (char)s[off + i];
        i++;
    }
    out[i] = 0;
    return i;
}

static void u64_dec(uint64_t v, char *out, int cap) {
    char tmp[32];
    int n = 0;
    if (v == 0) {
        tmp[n++] = '0';
    } else {
        while (v > 0 && n < 32) {
            tmp[n++] = (char)('0' + (v % 10));
            v /= 10;
        }
    }
    int o = 0;
    while (n > 0 && o + 1 < cap) {
        out[o++] = tmp[--n];
    }
    out[o] = 0;
}

static void hex16(const uint8_t *in, char *out) {
    static const char *d = "0123456789abcdef";
    for (int i = 0; i < 16; i++) {
        out[i * 2] = d[in[i] >> 4];
        out[i * 2 + 1] = d[in[i] & 0xf];
    }
    out[32] = 0;
}

static int emit(const char *restore, const char *time_s, const char *body) {
    char head[512];
    int n = 0;
    const char *p =
        "{\"v\":1,\"status\":200,\"headers\":["
        "[\"content-type\",\"text/plain\"],"
        "[\"x-raddy-restore\",\"";
    while (p[n] != 0) {
        head[n] = p[n];
        n++;
    }
    int i = 0;
    while (restore[i] != 0 && n < 500) {
        head[n++] = restore[i++];
    }
    const char *mid = "\"],[\"x-raddy-request-time\",\"";
    i = 0;
    while (mid[i] != 0 && n < 500) {
        head[n++] = mid[i++];
    }
    i = 0;
    while (time_s[i] != 0 && n < 500) {
        head[n++] = time_s[i++];
    }
    const char *tail = "\"]]}";
    i = 0;
    while (tail[i] != 0 && n < 500) {
        head[n++] = tail[i++];
    }
    if (raddy_resp_head((int32_t)(uintptr_t)head, n) < 0) {
        return 1;
    }
    int blen = 0;
    while (body[blen] != 0) {
        blen++;
    }
    if (blen > 0 && raddy_resp_write((int32_t)(uintptr_t)body, blen) < 0) {
        return 1;
    }
    if (raddy_resp_end() < 0) {
        return 1;
    }
    return 0;
}

__attribute__((export_name("raddy_execute")))
int32_t raddy_execute(void) {
    int32_t hlen = raddy_head_len();
    if (hlen < 0 || hlen > 32768) {
        return 1;
    }
    if (raddy_head_read((int32_t)(uintptr_t)heap, hlen) != hlen) {
        return 1;
    }

    char target[256];
    target[0] = 0;
    int tpos = find(heap, hlen, "\"target\":\"");
    if (tpos >= 0) {
        copy_until(heap, hlen, tpos + 10, '"', target, (int)sizeof(target));
    }

    uint8_t tbytes[8];
    if (wasi_clock_time_get(0, 1, (int32_t)(uintptr_t)tbytes) != 0) {
        return 1;
    }
    uint64_t ns = 0;
    for (int i = 0; i < 8; i++) {
        ns |= ((uint64_t)tbytes[i]) << (8 * i);
    }
    char time_s[32];
    u64_dec(ns / 1000000000ull, time_s, (int)sizeof(time_s));

    const char *restore = booted ? "snapshot" : "fresh";

    if (find((const uint8_t *)target, (int)sizeof(target), "/random") >= 0) {
        uint8_t rnd[16];
        if (wasi_random_get((int32_t)(uintptr_t)rnd, 16) != 0) {
            return 1;
        }
        char hex[33];
        hex16(rnd, hex);
        return emit(restore, time_s, hex);
    }

    char name[64];
    name[0] = 0;
    int npos = find((const uint8_t *)target, (int)sizeof(target), "name=");
    if (npos >= 0) {
        copy_until((const uint8_t *)target, (int)sizeof(target), npos + 5, '&', name, (int)sizeof(name));
    }
    char body[96];
    const char *prefix = "Hello ";
    int b = 0;
    while (prefix[b] != 0) {
        body[b] = prefix[b];
        b++;
    }
    int ni = 0;
    if (name[0] == 0) {
        body[b++] = 'x';
        body[b++] = 'o';
        body[b++] = 'r';
    } else {
        while (name[ni] != 0 && b < 90) {
            body[b++] = name[ni++];
        }
    }
    body[b] = 0;
    return emit(restore, time_s, body);
}
