#include "still_rt.h"
#include <math.h>
#include <time.h>
#include <errno.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <signal.h>
#include <unistd.h>
#endif

/* captured by still_args_init from the generated main() */
static int64_t g_still_argc = 0;
static char** g_still_argv = 0;

void still_panic(const char* file, int64_t line, const char* msg) {
    fflush(stdout);
    fprintf(stderr, "%s:%lld: panic: %s\n", file, (long long)line, msg);
    abort();
}

void still_check_fail(const char* file, int64_t line, const char* msg) {
    fflush(stdout);
    fprintf(stderr, "%s:%lld: check failed: %s\n", file, (long long)line, msg ? msg : "false condition");
    abort();
}

void still_fail(still_str* msg, const char* file, int64_t line) {
    fflush(stdout);
    fprintf(stderr, "%s:%lld: fail: %.*s\n", file, (long long)line, (int)msg->len, msg->bytes);
    abort();
}

void still_exit(int64_t code) {
    exit((int)code);
}

still_str* still_str_new(const char* s, int64_t len) {
    still_str* p = (still_str*)malloc(sizeof(still_str) + (size_t)len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    p->hdr.rc = 1;
    p->hdr.destroy = still_free_str;
    p->len = len;
    p->cap = len;
    if (len > 0) memcpy(p->bytes, s, (size_t)len);
    p->bytes[len] = 0;
    return p;
}

still_str* still_str_from_owned(char* buf, int64_t len) {
    still_str* p = (still_str*)malloc(sizeof(still_str) + (size_t)len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    p->hdr.rc = 1;
    p->hdr.destroy = still_free_str;
    p->len = len;
    p->cap = len;
    if (len > 0) memcpy(p->bytes, buf, (size_t)len);
    p->bytes[len] = 0;
    free(buf);
    return p;
}

void still_free_str(void* p) {
    free(p);
}

still_str* still_str_cat(still_str* a, still_str* b) {
    int64_t n = a->len + b->len;
    still_str* p = (still_str*)malloc(sizeof(still_str) + (size_t)n + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    p->hdr.rc = 1;
    p->hdr.destroy = still_free_str;
    p->len = n;
    p->cap = n;
    memcpy(p->bytes, a->bytes, (size_t)a->len);
    memcpy(p->bytes + a->len, b->bytes, (size_t)b->len);
    p->bytes[n] = 0;
    return p;
}

bool still_str_eq(still_str* a, still_str* b) {
    if (a == b) return true;
    if (!a || !b || a->len != b->len) return false;
    return memcmp(a->bytes, b->bytes, (size_t)a->len) == 0;
}

int64_t still_str_cmp(still_str* a, still_str* b) {
    int64_t n = a->len < b->len ? a->len : b->len;
    for (int64_t i = 0; i < n; i++) {
        if ((unsigned char)a->bytes[i] != (unsigned char)b->bytes[i]) {
            return (unsigned char)a->bytes[i] < (unsigned char)b->bytes[i] ? -1 : 1;
        }
    }
    if (a->len == b->len) return 0;
    return a->len < b->len ? -1 : 1;
}

still_str* still_str_map(still_str* a, int (*f)(int)) {
    still_str* p = still_str_new(a->bytes, a->len);
    for (int64_t i = 0; i < p->len; i++) p->bytes[i] = (char)f((unsigned char)p->bytes[i]);
    return p;
}

static int upper_c(int c) {
    return (c >= 'a' && c <= 'z') ? c - 32 : c;
}

static int lower_c(int c) {
    return (c >= 'A' && c <= 'Z') ? c + 32 : c;
}

still_str* still_str_upper(still_str* a) {
    return still_str_map(a, upper_c);
}

still_str* still_str_lower(still_str* a) {
    return still_str_map(a, lower_c);
}

still_str* still_str_trim(still_str* a) {
    int64_t s = 0;
    int64_t e = a->len;
    while (s < e && (a->bytes[s] == ' ' || a->bytes[s] == '\t' || a->bytes[s] == '\r' || a->bytes[s] == '\n')) s++;
    while (e > s && (a->bytes[e - 1] == ' ' || a->bytes[e - 1] == '\t' || a->bytes[e - 1] == '\r' || a->bytes[e - 1] == '\n')) e--;
    return still_str_new(a->bytes + s, e - s);
}

bool still_str_contains(still_str* a, still_str* b) {
    if (b->len == 0) return true;
    if (b->len > a->len) return false;
    for (int64_t i = 0; i + b->len <= a->len; i++) {
        if (memcmp(a->bytes + i, b->bytes, (size_t)b->len) == 0) return true;
    }
    return false;
}

bool still_str_starts(still_str* a, still_str* b) {
    if (b->len > a->len) return false;
    return memcmp(a->bytes, b->bytes, (size_t)b->len) == 0;
}

bool still_str_ends(still_str* a, still_str* b) {
    if (b->len > a->len) return false;
    return memcmp(a->bytes + (a->len - b->len), b->bytes, (size_t)b->len) == 0;
}

still_str* still_str_slice(still_str* a, int64_t start, int64_t end) {
    if (start < 0) start += a->len;
    if (end < 0) end += a->len;
    if (start < 0) start = 0;
    if (end > a->len) end = a->len;
    if (end < start) end = start;
    return still_str_new(a->bytes + start, end - start);
}

/* Python slice semantics with explicit step; STILL_SLICE_NONE = omitted bound */
still_str* still_str_slice3(still_str* a, int64_t start, int64_t end, int64_t step) {
    int64_t len = a->len;
    if (step > 0) {
        if (start == STILL_SLICE_NONE) start = 0; else if (start < 0) start += len;
        if (end == STILL_SLICE_NONE) end = len; else if (end < 0) end += len;
        if (start < 0) start = 0;
        if (start > len) start = len;
        if (end > len) end = len;
        still_sb* sb = still_sb_new();
        for (int64_t i = start; i < end; i += step) still_sb_c(sb, a->bytes[i]);
        still_str* out = still_str_new(sb->buf, sb->len);
        free(sb->buf);
        free(sb);
        return out;
    }
    if (start == STILL_SLICE_NONE) start = len - 1; else if (start < 0) start += len;
    if (end == STILL_SLICE_NONE) end = -1; else if (end < 0) end += len;
    if (start >= len) start = len - 1;
    if (end < -1) end = -1;
    still_sb* sb = still_sb_new();
    for (int64_t i = start; i > end; i += step) still_sb_c(sb, a->bytes[i]);
    still_str* out = still_str_new(sb->buf, sb->len);
    free(sb->buf);
    free(sb);
    return out;
}

still_str* still_str_index(still_str* a, int64_t i, const char* file, int64_t line) {
    if (i < 0) i += a->len;
    if (i < 0 || i >= a->len) still_panic(file, line, "index out of range");
    return still_str_new(a->bytes + i, 1);
}

still_str* still_str_from_int(int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof(buf), "%lld", (long long)v);
    return still_str_new(buf, n);
}

still_str* still_str_from_float(double v) {
    char buf[64];
    still_fmt_double(buf, v);
    return still_str_new(buf, (int64_t)strlen(buf));
}

still_str* still_str_from_bool(bool v) {
    return still_str_new(v ? "true" : "false", v ? 4 : 5);
}

int64_t still_str_to_int(still_str* s, const char* file, int64_t line) {
    char buf[64];
    if (s->len >= 63) still_panic(file, line, "cannot read a whole number from this string");
    memcpy(buf, s->bytes, (size_t)s->len);
    buf[s->len] = 0;
    char* end = NULL;
    errno = 0;
    long long v = strtoll(buf, &end, 10);
    if (end == buf || *end != 0 || errno != 0) still_panic(file, line, "cannot read a whole number from this string");
    return (int64_t)v;
}

double still_str_to_float(still_str* s, const char* file, int64_t line) {
    char buf[64];
    if (s->len >= 63) still_panic(file, line, "cannot read a number from this string");
    memcpy(buf, s->bytes, (size_t)s->len);
    buf[s->len] = 0;
    char* end = NULL;
    double v = strtod(buf, &end);
    if (end == buf || *end != 0) still_panic(file, line, "cannot read a number from this string");
    return v;
}

int64_t still_ord(still_str* s, const char* file, int64_t line) {
    if (s->len != 1) still_panic(file, line, "ord needs a one-character string");
    return (unsigned char)s->bytes[0];
}

still_str* still_chr(int64_t c, const char* file, int64_t line) {
    if (c < 0 || c > 255) still_panic(file, line, "chr works on values 0 to 255");
    char b = (char)c;
    return still_str_new(&b, 1);
}

void still_fmt_double(char* buf, double v) {
    if (v != v) { strcpy(buf, "nan"); return; }
    if (v > 1.7976931348623157e308 || v < -1.7976931348623157e308) { strcpy(buf, v > 0 ? "inf" : "-inf"); return; }
    if (v == (double)(int64_t)v && fabs(v) < 1e15) {
        snprintf(buf, 64, "%lld.0", (long long)v);
        return;
    }
    for (int prec = 1; prec <= 17; prec++) {
        snprintf(buf, 64, "%.*g", prec, v);
        if (strtod(buf, NULL) == v) return;
    }
}

int64_t still_ipow(int64_t a, int64_t b, const char* file, int64_t line) {
    if (b < 0) still_panic(file, line, "negative whole-number powers give fractions; use a float");
    int64_t r = 1;
    while (b > 0) {
        if (b & 1) r = still_mul(r, a, file, line);
        a = still_mul(a, a, file, line);
        b >>= 1;
    }
    return r;
}

int64_t still_iabs(int64_t a) {
    return a < 0 ? -a : a;
}

double still_floordivf(double a, double b) {
    return floor(a / b);
}

double still_fmodf_(double a, double b) {
    double r = fmod(a, b);
    if (r != 0 && ((r < 0) != (b < 0))) r += b;
    return r;
}

still_list_i* still_range(int64_t n) {
    still_list_i* l = still_list_i_alloc(n);
    for (int64_t i = 0; i < n; i++) still_list_i_push(l, i);
    return l;
}

still_list_i* still_range2(int64_t a, int64_t b) {
    still_list_i* l = still_list_i_alloc(b - a);
    if (a <= b) {
        for (int64_t i = a; i < b; i++) still_list_i_push(l, i);
    } else {
        for (int64_t i = a; i > b; i--) still_list_i_push(l, i);
    }
    return l;
}

still_list_i* still_range3(int64_t a, int64_t b, int64_t step) {
    still_list_i* l = still_list_i_alloc(8);
    if (step > 0) {
        for (int64_t i = a; i < b; i += step) still_list_i_push(l, i);
    } else {
        for (int64_t i = a; i > b; i += step) still_list_i_push(l, i);
    }
    return l;
}

static void still_list_i_destroy(void* p) {
    still_list_i* l = (still_list_i*)p;
    free(l->data);
    free(l);
}

static void still_list_s_destroy(void* p) {
    still_list_s* l = (still_list_s*)p;
    for (int64_t i = 0; i < l->len; i++) still_release(l->data[i]);
    free(l->data);
    free(l);
}

still_list_i* still_list_i_alloc(int64_t cap) {
    still_list_i* l = (still_list_i*)malloc(sizeof(still_list_i));
    if (!l) still_panic("memory", 0, "out of memory");
    l->hdr.rc = 1;
    l->hdr.destroy = still_list_i_destroy;
    l->len = 0;
    l->cap = cap > 0 ? cap : 8;
    l->data = (int64_t*)malloc((size_t)l->cap * sizeof(int64_t));
    if (!l->data) still_panic("memory", 0, "out of memory");
    return l;
}

still_list_s* still_list_s_alloc(int64_t cap) {
    still_list_s* l = (still_list_s*)malloc(sizeof(still_list_s));
    if (!l) still_panic("memory", 0, "out of memory");
    l->hdr.rc = 1;
    l->hdr.destroy = still_list_s_destroy;
    l->len = 0;
    l->cap = cap > 0 ? cap : 8;
    l->data = (still_str**)malloc((size_t)l->cap * sizeof(still_str*));
    if (!l->data) still_panic("memory", 0, "out of memory");
    return l;
}

void still_list_i_push(still_list_i* l, int64_t v) {
    if (l->len == l->cap) {
        l->cap *= 2;
        int64_t* nd = (int64_t*)realloc(l->data, (size_t)l->cap * sizeof(int64_t));
        if (!nd) still_panic("memory", 0, "out of memory");
        l->data = nd;
    }
    l->data[l->len++] = v;
}

void still_list_s_push(still_list_s* l, still_str* v) {
    if (l->len == l->cap) {
        l->cap *= 2;
        still_str** nd = (still_str**)realloc(l->data, (size_t)l->cap * sizeof(still_str*));
        if (!nd) still_panic("memory", 0, "out of memory");
        l->data = nd;
    }
    l->data[l->len++] = v;
}

int64_t still_list_i_get(still_list_i* l, int64_t i, const char* file, int64_t line) {
    if (i < 0) i += l->len;
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range");
    return l->data[i];
}

still_str* still_list_s_get(still_list_s* l, int64_t i, const char* file, int64_t line) {
    if (i < 0) i += l->len;
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range");
    return (still_str*)still_retain(l->data[i]);
}

void still_list_i_set(still_list_i* l, int64_t i, int64_t v, const char* file, int64_t line) {
    if (i < 0) i += l->len;
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range");
    l->data[i] = v;
}

void still_list_s_set(still_list_s* l, int64_t i, still_str* v, const char* file, int64_t line) {
    if (i < 0) i += l->len;
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range");
    still_release(l->data[i]);
    l->data[i] = v;
}

int64_t still_list_i_pop(still_list_i* l, const char* file, int64_t line) {
    if (l->len == 0) still_panic(file, line, "cannot pop from an empty list");
    return l->data[--l->len];
}

still_str* still_list_s_pop(still_list_s* l, const char* file, int64_t line) {
    if (l->len == 0) still_panic(file, line, "cannot pop from an empty list");
    return l->data[--l->len];
}

void still_list_i_clear(still_list_i* l) {
    l->len = 0;
}

void still_list_s_clear(still_list_s* l) {
    for (int64_t i = 0; i < l->len; i++) still_release(l->data[i]);
    l->len = 0;
}

still_list_i* still_list_i_concat(still_list_i* a, still_list_i* b) {
    still_list_i* r = still_list_i_alloc(a->len + b->len);
    for (int64_t i = 0; i < a->len; i++) still_list_i_push(r, a->data[i]);
    for (int64_t i = 0; i < b->len; i++) still_list_i_push(r, b->data[i]);
    return r;
}

still_list_s* still_list_s_concat(still_list_s* a, still_list_s* b) {
    still_list_s* r = still_list_s_alloc(a->len + b->len);
    for (int64_t i = 0; i < a->len; i++) still_list_s_push(r, (still_str*)still_retain(a->data[i]));
    for (int64_t i = 0; i < b->len; i++) still_list_s_push(r, (still_str*)still_retain(b->data[i]));
    return r;
}

bool still_list_i_eq(still_list_i* a, still_list_i* b) {
    if (a == b) return true;
    if (!a || !b || a->len != b->len) return false;
    return memcmp(a->data, b->data, (size_t)a->len * sizeof(int64_t)) == 0;
}

bool still_list_s_eq(still_list_s* a, still_list_s* b) {
    if (a == b) return true;
    if (!a || !b || a->len != b->len) return false;
    for (int64_t i = 0; i < a->len; i++) {
        if (!still_str_eq(a->data[i], b->data[i])) return false;
    }
    return true;
}

bool still_list_i_contains(still_list_i* l, int64_t v) {
    for (int64_t i = 0; i < l->len; i++) {
        if (l->data[i] == v) return true;
    }
    return false;
}

bool still_list_s_contains(still_list_s* l, still_str* v) {
    for (int64_t i = 0; i < l->len; i++) {
        if (still_str_eq(l->data[i], v)) return true;
    }
    return false;
}

still_list_i* still_list_i_slice(still_list_i* l, int64_t a, int64_t b) {
    if (a < 0) a += l->len;
    if (b < 0) b += l->len;
    if (a < 0) a = 0;
    if (b > l->len) b = l->len;
    if (b < a) b = a;
    still_list_i* r = still_list_i_alloc(b - a);
    for (int64_t i = a; i < b; i++) still_list_i_push(r, l->data[i]);
    return r;
}

still_list_i* still_list_i_slice3(still_list_i* l, int64_t start, int64_t end, int64_t step) {
    int64_t len = l->len;
    if (step > 0) {
        if (start == STILL_SLICE_NONE) start = 0; else if (start < 0) start += len;
        if (end == STILL_SLICE_NONE) end = len; else if (end < 0) end += len;
        if (start < 0) start = 0;
        if (start > len) start = len;
        if (end > len) end = len;
    } else {
        if (start == STILL_SLICE_NONE) start = len - 1; else if (start < 0) start += len;
        if (end == STILL_SLICE_NONE) end = -1; else if (end < 0) end += len;
        if (start >= len) start = len - 1;
        if (end < -1) end = -1;
    }
    still_list_i* r = still_list_i_alloc(8);
    if (step > 0) {
        for (int64_t i = start; i < end; i += step) still_list_i_push(r, l->data[i]);
    } else {
        for (int64_t i = start; i > end; i += step) still_list_i_push(r, l->data[i]);
    }
    return r;
}

still_list_s* still_list_s_slice(still_list_s* l, int64_t a, int64_t b) {
    if (a < 0) a += l->len;
    if (b < 0) b += l->len;
    if (a < 0) a = 0;
    if (b > l->len) b = l->len;
    if (b < a) b = a;
    still_list_s* r = still_list_s_alloc(b - a);
    for (int64_t i = a; i < b; i++) still_list_s_push(r, (still_str*)still_retain(l->data[i]));
    return r;
}

still_list_s* still_list_s_slice3(still_list_s* l, int64_t start, int64_t end, int64_t step) {
    int64_t len = l->len;
    if (step > 0) {
        if (start == STILL_SLICE_NONE) start = 0; else if (start < 0) start += len;
        if (end == STILL_SLICE_NONE) end = len; else if (end < 0) end += len;
        if (start < 0) start = 0;
        if (start > len) start = len;
        if (end > len) end = len;
    } else {
        if (start == STILL_SLICE_NONE) start = len - 1; else if (start < 0) start += len;
        if (end == STILL_SLICE_NONE) end = -1; else if (end < 0) end += len;
        if (start >= len) start = len - 1;
        if (end < -1) end = -1;
    }
    still_list_s* r = still_list_s_alloc(8);
    if (step > 0) {
        for (int64_t i = start; i < end; i += step) still_list_s_push(r, (still_str*)still_retain(l->data[i]));
    } else {
        for (int64_t i = start; i > end; i += step) still_list_s_push(r, (still_str*)still_retain(l->data[i]));
    }
    return r;
}

void still_args_init(int argc, char** argv) {
    g_still_argc = (int64_t)argc;
    g_still_argv = argv;
}

still_list_s* still_args(void) {
    still_list_s* r = still_list_s_alloc(g_still_argc > 0 ? g_still_argc : 0);
    for (int64_t i = 0; i < g_still_argc; i++) {
        if (g_still_argv[i]) {
            still_list_s_push(r, still_str_new(g_still_argv[i], (int64_t)strlen(g_still_argv[i])));
        }
    }
    return r;
}

still_list_i* still_list_i_reversed(still_list_i* l) {
    still_list_i* r = still_list_i_alloc(l->len);
    for (int64_t i = l->len - 1; i >= 0; i--) still_list_i_push(r, l->data[i]);
    return r;
}

still_list_s* still_list_s_reversed(still_list_s* l) {
    still_list_s* r = still_list_s_alloc(l->len);
    for (int64_t i = l->len - 1; i >= 0; i--) still_list_s_push(r, (still_str*)still_retain(l->data[i]));
    return r;
}

void still_print_str(still_str* s) {
    fwrite(s->bytes, 1, (size_t)s->len, stdout);
}

void still_print_int(int64_t v) {
    printf("%lld", (long long)v);
}

void still_print_float(double v) {
    char buf[64];
    still_fmt_double(buf, v);
    fputs(buf, stdout);
}

void still_print_bool(bool v) {
    fputs(v ? "true" : "false", stdout);
}

void still_print_none(void) {
    fputs("none", stdout);
}

void still_print_sep(void) {
    fputc(' ', stdout);
}

void still_print_nl(void) {
    fputc('\n', stdout);
}

still_str* still_input(still_str* prompt) {
    if (prompt) {
        fwrite(prompt->bytes, 1, (size_t)prompt->len, stdout);
        fflush(stdout);
    }
    size_t cap = 128;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    if (!buf) still_panic("memory", 0, "out of memory");
    int c;
    while ((c = fgetc(stdin)) != EOF && c != '\n') {
        if (len + 1 >= cap) {
            cap *= 2;
            char* nb = (char*)realloc(buf, cap);
            if (!nb) still_panic("memory", 0, "out of memory");
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    still_str* r = still_str_new(buf, (int64_t)len);
    free(buf);
    return r;
}

double still_clock(void) {
#ifdef _WIN32
    FILETIME ft;
    ULARGE_INTEGER u;
    GetSystemTimeAsFileTime(&ft);
    u.LowPart = ft.dwLowDateTime;
    u.HighPart = ft.dwHighDateTime;
    return (double)u.QuadPart / 10000000.0 - 11644473600.0;
#else
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1000000000.0;
#endif
}

static uint64_t still_rng_state = 0;

static uint64_t still_xorshift(uint64_t* s) {
    uint64_t x = *s;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    return x;
}

int64_t still_rand(void) {
    if (!still_rng_state) {
        uint64_t pid =
#ifdef _WIN32
            (uint64_t)GetCurrentProcessId();
#else
            (uint64_t)getpid();
#endif
        still_rng_state = 0x9E3779B97F4A7C15ULL ^ (uint64_t)time(NULL) ^ (pid << 32);
    }
    return (int64_t)(still_xorshift(&still_rng_state) >> 1);
}

int64_t still_rand_bound(int64_t n) {
    if (n <= 0) return 0;
    return (int64_t)(still_rand() % n);
}

void still_seed(int64_t s) {
    still_rng_state = (uint64_t)s ? (uint64_t)s : 0x9E3779B97F4A7C15ULL;
}

still_str* still_read_file(still_str* path) {
    char* p = (char*)malloc((size_t)path->len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, path->bytes, (size_t)path->len);
    p[path->len] = 0;
    FILE* f = fopen(p, "rb");
    free(p);
    if (!f) still_panic("file", 0, "cannot open this file for reading");
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* buf = (char*)malloc((size_t)n + 1);
    if (!buf) {
        fclose(f);
        still_panic("memory", 0, "out of memory");
    }
    size_t got = fread(buf, 1, (size_t)n, f);
    fclose(f);
    still_str* r = still_str_from_owned(buf, (int64_t)got);
    return r;
}

int64_t still_write_file(still_str* path, still_str* data) {
    char* p = (char*)malloc((size_t)path->len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, path->bytes, (size_t)path->len);
    p[path->len] = 0;
    FILE* f = fopen(p, "wb");
    free(p);
    if (!f) still_panic("file", 0, "cannot open this file for writing");
    size_t wrote = fwrite(data->bytes, 1, (size_t)data->len, f);
    fclose(f);
    return (int64_t)wrote;
}

still_str* still_get_env(still_str* name) {
    char* p = (char*)malloc((size_t)name->len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, name->bytes, (size_t)name->len);
    p[name->len] = 0;
    const char* v = getenv(p);
    free(p);
    if (!v) return NULL;
    return still_str_new(v, (int64_t)strlen(v));
}

void still_sleep_ms(int64_t ms) {
#ifdef _WIN32
    Sleep((DWORD)ms);
#else
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000;
    nanosleep(&ts, NULL);
#endif
}

int64_t still_soa_index(int64_t i, int64_t len, const char* file, int64_t line) {
    if (i < 0) i += len;
    if (i < 0 || i >= len) still_panic(file, line, "index out of range");
    return i;
}

int64_t still_dict_next_cap(int64_t want) {
    int64_t c = 8;
    while (c < want * 2) c *= 2;
    return c;
}

uint64_t still_hash_i64(int64_t v) {
    uint64_t x = (uint64_t)v;
    x += 0x9E3779B97F4A7C15ULL;
    x = (x ^ (x >> 30)) * 0xBF58476D1CE4E5B9ULL;
    x = (x ^ (x >> 27)) * 0x94D049BB133111EBULL;
    x = x ^ (x >> 31);
    return x;
}

uint64_t still_hash_str(still_str* s) {
    uint64_t h = 14695981039346656037ULL;
    for (int64_t i = 0; i < s->len; i++) {
        h ^= (unsigned char)s->bytes[i];
        h *= 1099511628211ULL;
    }
    return h;
}

bool still_eq_i64(int64_t a, int64_t b) { return a == b; }
bool still_eq_f64(double a, double b) { return a == b; }
bool still_eq_bool(bool a, bool b) { return a == b; }

static bool still_bp_any = false;
static char still_bp_files[64][256];
static int64_t still_bp_lines[64];
static int still_bp_count = 0;

static void still_bp_init(void) {
    const char* env = getenv("STILL_BREAKPOINTS");
    if (!env || !env[0]) return;
    const char* p = env;
    while (*p && still_bp_count < 64) {
        char* f = still_bp_files[still_bp_count];
        int fi = 0;
        while (*p && *p != ':' && fi < 255) f[fi++] = *p++;
        f[fi] = 0;
        if (*p != ':') break;
        p++;
        still_bp_lines[still_bp_count] = strtoll(p, (char**)&p, 10);
        if (*p == ',') p++;
        still_bp_count++;
    }
    still_bp_any = still_bp_count > 0;
}

void still_bp_check(const char* file, int64_t line) {
    if (!still_bp_any) return;
    const char* base = strrchr(file, '/');
#ifdef _WIN32
    const char* base2 = strrchr(file, '\\');
    if (base2 > base) base = base2;
#endif
    if (base) base++; else base = file;
    for (int i = 0; i < still_bp_count; i++) {
        if (still_bp_lines[i] == line && (strcmp(still_bp_files[i], file) == 0 || strcmp(still_bp_files[i], base) == 0)) {
            fflush(stdout);
            fprintf(stderr, "breakpoint hit at %s:%lld\n", file, (long long)line);
#ifdef _WIN32
            DebugBreak();
#else
            raise(SIGTRAP);
#endif
        }
    }
}

void still_init(void) {
    still_bp_init();
}

still_list_s* still_str_split(still_str* s, still_str* sep) {
    still_list_s* r = still_list_s_alloc(8);
    if (sep->len == 0) {
        still_list_s_push(r, (still_str*)still_retain(s));
        return r;
    }
    if (s->len == 0) {
        still_list_s_push(r, still_str_new("", 0));
        return r;
    }
    int64_t start = 0;
    int64_t i = 0;
    while (i + sep->len <= s->len) {
        if (memcmp(s->bytes + i, sep->bytes, (size_t)sep->len) == 0) {
            still_list_s_push(r, still_str_new(s->bytes + start, i - start));
            i += sep->len;
            start = i;
        } else {
            i++;
        }
    }
    still_list_s_push(r, still_str_new(s->bytes + start, s->len - start));
    return r;
}

still_str* still_str_replace(still_str* s, still_str* from, still_str* to) {
    if (from->len == 0 || s->len == 0) {
        return (still_str*)still_retain(s);
    }
    still_sb* sb = still_sb_new();
    int64_t i = 0;
    while (i < s->len) {
        if (i + from->len <= s->len && memcmp(s->bytes + i, from->bytes, (size_t)from->len) == 0) {
            still_sb_s(sb, to);
            i += from->len;
        } else {
            still_sb_c(sb, s->bytes[i]);
            i++;
        }
    }
    still_str* out = still_str_new(sb->buf, sb->len);
    free(sb->buf);
    free(sb);
    return out;
}

bool still_ptr_eq(void* a, void* b) {
    return a == b;
}

still_sb* still_sb_new(void) {
    still_sb* sb = (still_sb*)malloc(sizeof(still_sb));
    if (!sb) still_panic("memory", 0, "out of memory");
    sb->cap = 64;
    sb->len = 0;
    sb->buf = (char*)malloc(64);
    if (!sb->buf) still_panic("memory", 0, "out of memory");
    return sb;
}

void still_sb_c(still_sb* sb, char c) {
    if (sb->len + 1 >= sb->cap) {
        sb->cap *= 2;
        char* nb = (char*)realloc(sb->buf, (size_t)sb->cap);
        if (!nb) still_panic("memory", 0, "out of memory");
        sb->buf = nb;
    }
    sb->buf[sb->len++] = c;
}

void still_sb_s(still_sb* sb, still_str* s) {
    for (int64_t i = 0; i < s->len; i++) {
        still_sb_c(sb, s->bytes[i]);
    }
}

void still_sb_raw(still_sb* sb, const char* s) {
    while (*s) {
        still_sb_c(sb, *s++);
    }
}

still_str* still_sb_finish(still_sb* sb) {
    still_str* r = still_str_new(sb->buf, sb->len);
    free(sb->buf);
    free(sb);
    return r;
}

void still_sb_add_int(still_sb* sb, int64_t v) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%lld", (long long)v);
    still_sb_raw(sb, buf);
}

void still_sb_add_float(still_sb* sb, double v) {
    char buf[64];
    still_fmt_double(buf, v);
    still_sb_raw(sb, buf);
}

void still_sb_add_bool(still_sb* sb, bool v) {
    still_sb_raw(sb, v ? "true" : "false");
}

void still_sb_add_str(still_sb* sb, still_str* v) {
    still_sb_s(sb, v);
}

void still_sb_add_none(still_sb* sb) {
    still_sb_raw(sb, "none");
}

void still_tostr_add_soa_stub(still_sb* sb, void* p) {
    if (!p) {
        still_sb_raw(sb, "none");
        return;
    }
    still_soa_hdr* h = (still_soa_hdr*)p;
    still_sb_raw(sb, "soa(");
    still_sb_add_int(sb, h->len);
    still_sb_c(sb, ')');
}
