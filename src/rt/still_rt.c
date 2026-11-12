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

still_str* still_str_join(still_str* sep, still_list_s* parts) {
    still_sb* sb = still_sb_new();
    for (int64_t i = 0; i < parts->len; i++) {
        if (i > 0 && sep->len > 0) still_sb_s(sb, sep);
        still_str* p = parts->data[i];
        if (p->len > 0) still_sb_s(sb, p);
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

/* ---------------------------------------------------------------- Python bridge */
/* CPython embedded at run time: libpython is loaded via dlopen/LoadLibrary, so
   programs that never use py have no link-time dependency on it.
   STILL_PYTHON_DLL overrides the built-in search list. The bridge is
   single-threaded: the main thread holds the GIL after init; do not call py
   functions from other threads. */

#ifdef _WIN32
static HMODULE g_py_dll = NULL;
#else
#include <dlfcn.h>
static void* g_py_dll = NULL;
#endif

static int g_py_syms_ok = 0;

static int (*p_Py_IsInitialized)(void);
static void (*p_Py_InitializeEx)(int);
static int (*p_PyRun_SimpleString)(const char*);
static void* (*p_PyRun_String)(const char*, int, void*, void*);
static void* (*p_PyImport_ImportModule)(const char*);
static void* (*p_PyModule_GetDict)(void*);
static void* (*p_PyObject_Str)(void*);
static const char* (*p_PyUnicode_AsUTF8)(void*);
static void (*p_Py_DecRef)(void*);
static void* (*p_PyErr_Occurred)(void);
static void (*p_PyErr_Print)(void);
static void (*p_PyErr_Fetch)(void**, void**, void**);
static void (*p_PyErr_NormalizeException)(void**, void**, void**);
static void* (*p_PyObject_GetAttrString)(void*, const char*);

static void* py_sym(const char* name) {
#ifdef _WIN32
    return (void*)GetProcAddress(g_py_dll, name);
#else
    return dlsym(g_py_dll, name);
#endif
}

static int py_load(void) {
    if (g_py_dll) return 1;
    const char* ov = getenv("STILL_PYTHON_DLL");
    if (ov && ov[0]) {
#ifdef _WIN32
        g_py_dll = LoadLibraryA(ov);
#else
        g_py_dll = dlopen(ov, RTLD_NOW | RTLD_GLOBAL);
#endif
        if (g_py_dll) return 1;
    }
#ifdef _WIN32
    /* newest first; python3.dll (stable ABI forwarder) last as a fallback */
    static const char* cands[] = {
        "python314.dll", "python313.dll", "python312.dll", "python311.dll",
        "python310.dll", "python39.dll", "python3.dll", NULL,
    };
    for (int i = 0; cands[i]; i++) {
        g_py_dll = LoadLibraryA(cands[i]);
        if (g_py_dll) return 1;
    }
#else
    /* newest first; runtime soname, then the dev symlink, then the macOS name */
    static const char* vers[] = { "14", "13", "12", "11", "10", "9", "8", NULL };
    char buf[64];
    for (int i = 0; vers[i]; i++) {
        snprintf(buf, sizeof(buf), "libpython3.%s.so.1.0", vers[i]);
        g_py_dll = dlopen(buf, RTLD_NOW | RTLD_GLOBAL);
        if (g_py_dll) return 1;
        snprintf(buf, sizeof(buf), "libpython3.%s.so", vers[i]);
        g_py_dll = dlopen(buf, RTLD_NOW | RTLD_GLOBAL);
        if (g_py_dll) return 1;
        snprintf(buf, sizeof(buf), "libpython3.%s.dylib", vers[i]);
        g_py_dll = dlopen(buf, RTLD_NOW | RTLD_GLOBAL);
        if (g_py_dll) return 1;
    }
    g_py_dll = dlopen("libpython3.so", RTLD_NOW | RTLD_GLOBAL);
    if (g_py_dll) return 1;
#endif
    return 0;
}

static int py_syms(void) {
    if (g_py_syms_ok) return 1;
    struct { const char* name; void** slot; } tab[] = {
        { "Py_IsInitialized", (void**)&p_Py_IsInitialized },
        { "Py_InitializeEx", (void**)&p_Py_InitializeEx },
        { "PyRun_SimpleString", (void**)&p_PyRun_SimpleString },
        { "PyRun_String", (void**)&p_PyRun_String },
        { "PyImport_ImportModule", (void**)&p_PyImport_ImportModule },
        { "PyModule_GetDict", (void**)&p_PyModule_GetDict },
        { "PyObject_Str", (void**)&p_PyObject_Str },
        { "PyUnicode_AsUTF8", (void**)&p_PyUnicode_AsUTF8 },
        { "Py_DecRef", (void**)&p_Py_DecRef },
        { "PyErr_Occurred", (void**)&p_PyErr_Occurred },
        { "PyErr_Print", (void**)&p_PyErr_Print },
        { "PyErr_Fetch", (void**)&p_PyErr_Fetch },
        { "PyErr_NormalizeException", (void**)&p_PyErr_NormalizeException },
        { "PyObject_GetAttrString", (void**)&p_PyObject_GetAttrString },
    };
    for (int i = 0; i < (int)(sizeof(tab) / sizeof(tab[0])); i++) {
        *tab[i].slot = py_sym(tab[i].name);
        if (!*tab[i].slot) return 0;
    }
    g_py_syms_ok = 1;
    return 1;
}

static int py_ensure_init(void) {
    if (!py_load()) return 0;
    if (!py_syms()) return 0;
    if (!p_Py_IsInitialized()) p_Py_InitializeEx(0);
    return p_Py_IsInitialized() ? 1 : 0;
}

int64_t still_py_init(void) {
    if (!py_ensure_init()) {
        fprintf(stderr, "still: python bridge: could not load libpython (set STILL_PYTHON_DLL)\n");
        return -1;
    }
    return 0;
}

int64_t still_py_run(const char* code) {
    if (!py_ensure_init()) return -1;
    if (p_PyRun_SimpleString(code) != 0) {
        if (p_PyErr_Occurred()) p_PyErr_Print();
        return -1;
    }
    return 0;
}

const char* still_py_eval(const char* expr) {
    if (!py_ensure_init()) still_panic("python", 0, "cannot load libpython (set STILL_PYTHON_DLL)");
    void* mod = p_PyImport_ImportModule("__main__");
    if (!mod) still_panic("python", 0, "cannot get the __main__ module");
    void* dict = p_PyModule_GetDict(mod);
    void* r = p_PyRun_String(expr, 258 /* Py_eval_input */, dict, dict);
    p_Py_DecRef(mod);
    if (!r) {
        void* t = NULL, *v = NULL, *tb = NULL;
        char tn[64] = "Error";
        char vm[256] = "";
        p_PyErr_Fetch(&t, &v, &tb);
        p_PyErr_NormalizeException(&t, &v, &tb);
        if (t) {
            void* name = p_PyObject_GetAttrString(t, "__name__");
            if (name) {
                const char* s = p_PyUnicode_AsUTF8(name);
                if (s) snprintf(tn, sizeof(tn), "%s", s);
                p_Py_DecRef(name);
            }
            p_Py_DecRef(t);
        }
        if (v) {
            void* so = p_PyObject_Str(v);
            if (so) {
                const char* s = p_PyUnicode_AsUTF8(so);
                if (s) snprintf(vm, sizeof(vm), "%s", s);
                p_Py_DecRef(so);
            }
            p_Py_DecRef(v);
        }
        if (tb) p_Py_DecRef(tb);
        char msg[384];
        snprintf(msg, sizeof(msg), "%s: %s", tn, vm[0] ? vm : "<no message>");
        still_panic("python", 0, msg);
    }
    void* so = p_PyObject_Str(r);
    const char* s = so ? p_PyUnicode_AsUTF8(so) : "";
    int64_t n = (int64_t)strlen(s);
    static char* eval_buf = NULL;
    static int64_t eval_cap = 0;
    if (n + 1 > eval_cap) {
        eval_cap = n + 64;
        char* nb = (char*)realloc(eval_buf, (size_t)eval_cap);
        if (!nb) still_panic("memory", 0, "out of memory");
        eval_buf = nb;
    }
    memcpy(eval_buf, s, (size_t)n + 1);
    if (so) p_Py_DecRef(so);
    return eval_buf;
}

// ===================== any =====================

static void still_any_destroy(void* p) {
    still_any* a = (still_any*)p;
    int c = (int)(a->tag & 0xFF);
    if (c == 's' || c == 'l' || c == 'd') still_release(a->u.p);
    free(p);
}

static still_any* still_any_alloc(int64_t tag) {
    still_any* a = (still_any*)malloc(sizeof(still_any));
    if (!a) still_panic("memory", 0, "out of memory");
    a->hdr.rc = 1;
    a->hdr.destroy = still_any_destroy;
    a->tag = tag;
    a->u.p = NULL;
    return a;
}

still_any* still_any_none(void) { return still_any_alloc('n'); }

still_any* still_any_new_i(int64_t v) {
    still_any* a = still_any_alloc('i');
    a->u.i = v;
    return a;
}

still_any* still_any_new_f(double v) {
    still_any* a = still_any_alloc('f');
    a->u.f = v;
    return a;
}

still_any* still_any_new_b(int64_t v) {
    still_any* a = still_any_alloc('b');
    a->u.i = v ? 1 : 0;
    return a;
}

still_any* still_any_new_s(still_str* s) {
    still_any* a = still_any_alloc('s');
    a->u.p = still_retain(s);
    return a;
}

still_any* still_any_new_p(void* p, int64_t tag) {
    still_any* a = still_any_alloc(tag);
    a->u.p = still_retain(p);
    return a;
}

// ---- raw container geometry (mirrors the layouts the compiler emits) ----
// list: hdr(0..16) len@16 cap@24 data@32 ; dict: hdr len@16 cap@24 used@32 slots@40
static int64_t any_len_raw(void* p) { return *(int64_t*)((char*)p + 16); }
static void* any_list_data(void* p) { return *(void**)((char*)p + 32); }
static int64_t any_dict_cap(void* p) { return *(int64_t*)((char*)p + 24); }
static void* any_dict_slots(void* p) { return *(void**)((char*)p + 40); }

static int any_tag_size(int64_t tag) {
    int c = (int)(tag & 0xFF);
    if (c == 'l') return 1 + any_tag_size(tag >> 8);
    if (c == 'd') {
        int ks = any_tag_size(tag >> 8);
        return 1 + ks + any_tag_size(tag >> (8 * (1 + ks)));
    }
    return 1;
}

// slot geometry for a dict whose packed tag is `tag`
static void any_dict_geo(int64_t tag, int64_t* stride, int64_t* koff, int64_t* voff) {
    int64_t ks = any_tag_size(tag >> 8);
    int kt = (int)((tag >> 8) & 0xFF);
    int64_t vs = 8 * (1 + ks);
    int vt = (int)((tag >> vs) & 0xFF);
    int64_t ksz = (kt == 'b') ? 1 : 8;
    int64_t vsz = (vt == 'b') ? 1 : 8;
    int64_t kalign = (kt == 'b') ? 1 : 8;
    int64_t valign = (vt == 'b') ? 1 : 8;
    int64_t ko = kalign; // align_up(1, kalign) after the 1-byte state field
    int64_t vo = ((ko + ksz) + valign - 1) / valign * valign;
    int64_t align = kalign > valign ? kalign : valign;
    *stride = ((vo + vsz) + align - 1) / align * align;
    *koff = ko;
    *voff = vo;
}

static int64_t any_stride_of(int64_t ctag) { return ((ctag & 0xFF) == 'b') ? 1 : 8; }

// ---- equality ----

static int any_cmp_eq(int64_t ctag, void* b1, int64_t st1, int64_t o1, int64_t i1,
                      void* b2, int64_t st2, int64_t o2, int64_t i2) {
    char* p = (char*)b1 + i1 * st1 + o1;
    char* q = (char*)b2 + i2 * st2 + o2;
    int c = (int)(ctag & 0xFF);
    if (c == 'a') return still_any_eq(*(still_any**)p, *(still_any**)q);
    if (c == 's') return still_str_eq(*(still_str**)p, *(still_str**)q);
    if (c == 'l' || c == 'd') return any_raw_eq(ctag, *(void**)p, *(void**)q);
    if (c == 'b') return (*(unsigned char*)p != 0) == (*(unsigned char*)q != 0);
    if (c == 'f') return *(double*)p == *(double*)q;
    return *(int64_t*)p == *(int64_t*)q;
}

static int any_raw_eq(int64_t tag, void* p, void* q) {
    if (p == q) return 1;
    if (!p || !q) return 0;
    int c = (int)(tag & 0xFF);
    if (c == 'l') {
        int64_t ct = tag >> 8;
        if (any_len_raw(p) != any_len_raw(q)) return 0;
        void* da = any_list_data(p);
        void* db = any_list_data(q);
        int64_t st = any_stride_of(ct);
        for (int64_t i = 0; i < any_len_raw(p); i++) {
            if (!any_cmp_eq(ct, da, st, 0, i, db, st, 0, i)) return 0;
        }
        return 1;
    }
    if (c == 'd') {
        int64_t ks = any_tag_size(tag >> 8);
        int64_t kt = tag >> 8;
        int64_t vt = tag >> (8 * (1 + ks));
        if (any_len_raw(p) != any_len_raw(q)) return 0;
        int64_t stride, koff, voff;
        any_dict_geo(tag, &stride, &koff, &voff);
        void* sa = any_dict_slots(p);
        void* sb = any_dict_slots(q);
        for (int64_t i = 0; i < any_dict_cap(p); i++) {
            if (*((unsigned char*)sa + i * stride) != 1) continue;
            int found = 0;
            for (int64_t j = 0; j < any_dict_cap(q); j++) {
                if (*((unsigned char*)sb + j * stride) != 1) continue;
                if (any_cmp_eq(kt, sa, stride, koff, i, sb, stride, koff, j)) {
                    found = any_cmp_eq(vt, sa, stride, voff, i, sb, stride, voff, j);
                    break;
                }
            }
            if (!found) return 0;
        }
        return 1;
    }
    return 0;
}

int still_any_eq(still_any* a, still_any* b) {
    if (a == b) return 1;
    // NULL is a valid representation of "the none any" (from zero-init), so
    // tag extraction must be NULL-safe.
    int ca = a ? (int)(a->tag & 0xFF) : 'n';
    int cb = b ? (int)(b->tag & 0xFF) : 'n';
    if (ca == 'n' && cb == 'n') return 1;
    if (ca == 'n' || cb == 'n') return 0;
    if (!a || !b) return 0;
    int64_t ta = a->tag, tb = b->tag;
    if (ca == 'i' && cb == 'f') return (double)a->u.i == b->u.f;
    if (ca == 'f' && cb == 'i') return a->u.f == (double)b->u.i;
    if (ta != tb) return 0;
    if (ca == 'i' || ca == 'b') return a->u.i == b->u.i;
    if (ca == 'f') return a->u.f == b->u.f;
    if (ca == 's') return still_str_eq((still_str*)a->u.p, (still_str*)b->u.p);
    if (ca == 'l' || ca == 'd') return any_raw_eq(ta, a->u.p, b->u.p);
    return 0;
}

// ---- truthiness / ordering ----

int still_any_truth(still_any* a) {
    if (!a || (a->tag & 0xFF) == 'n') return 0;
    switch (a->tag & 0xFF) {
        case 'i': case 'b': return a->u.i != 0;
        case 'f': return a->u.f != 0.0;
        case 's': return a->u.p && ((still_str*)a->u.p)->len > 0;
        case 'l': case 'd': return a->u.p && any_len_raw(a->u.p) > 0;
    }
    return 1;
}

static double any_num_f(still_any* a) {
    return ((a->tag & 0xFF) == 'f') ? a->u.f : (double)a->u.i;
}

static int any_order(int64_t r, int op) {
    switch (op) {
        case 2: return r < 0;
        case 3: return r <= 0;
        case 4: return r > 0;
        default: return r >= 0;
    }
}

static int any_cmp_child(int64_t ctag, void* b1, int64_t st1, int64_t o1, int64_t i1,
                         void* b2, int64_t st2, int64_t o2, int64_t i2) {
    char* p = (char*)b1 + i1 * st1 + o1;
    char* q = (char*)b2 + i2 * st2 + o2;
    int c = (int)(ctag & 0xFF);
    if (c == 'i' || c == 'b' || c == 'f') {
        double x = (c == 'f') ? *(double*)p : (c == 'b') ? (double)(*(unsigned char*)p != 0) : (double)*(int64_t*)p;
        double y = (c == 'f') ? *(double*)q : (c == 'b') ? (double)(*(unsigned char*)q != 0) : (double)*(int64_t*)q;
        return x < y ? -1 : (x > y ? 1 : 0);
    }
    if (c == 's') {
        int64_t r = still_str_cmp(*(still_str**)p, *(still_str**)q);
        return r < 0 ? -1 : (r > 0 ? 1 : 0);
    }
    if (c == 'l') return any_raw_cmp(ctag, *(void**)p, *(void**)q);
    still_panic("still", 0, "TypeError: this value is not orderable");
    return 0;
}

static int any_raw_cmp(int64_t tag, void* p, void* q) {
    if (p == q) return 0;
    if (!p || !q) still_panic("still", 0, "TypeError: cannot order none");
    int64_t ct = tag >> 8;
    int64_t n = any_len_raw(p), m = any_len_raw(q);
    void* da = any_list_data(p);
    void* db = any_list_data(q);
    int64_t st = any_stride_of(ct);
    int64_t lim = n < m ? n : m;
    for (int64_t i = 0; i < lim; i++) {
        int r = any_cmp_child(ct, da, st, 0, i, db, st, 0, i);
        if (r) return r;
    }
    return n == m ? 0 : (n < m ? -1 : 1);
}

int still_any_cmp(int op, still_any* a, still_any* b) {
    if (op == 0) return still_any_eq(a, b);
    if (op == 1) return !still_any_eq(a, b);
    if (!a || !b) still_panic("still", 0, "TypeError: cannot order none values");
    int ca = (int)(a->tag & 0xFF), cb = (int)(b->tag & 0xFF);
    if ((ca == 'i' || ca == 'b' || ca == 'f') && (cb == 'i' || cb == 'b' || cb == 'f')) {
        double x = any_num_f(a), y = any_num_f(b);
        int64_t r = x < y ? -1 : (x > y ? 1 : 0);
        return any_order(r, op);
    }
    if (ca == 's' && cb == 's') return any_order(still_str_cmp((still_str*)a->u.p, (still_str*)b->u.p), op);
    if (ca == 'l' && cb == 'l' && a->tag == b->tag) return any_order(any_raw_cmp(a->tag, a->u.p, b->u.p), op);
    char msg[160];
    snprintf(msg, sizeof(msg), "TypeError: '%s' and '%s' cannot be ordered", still_any_typename(a), still_any_typename(b));
    still_panic("still", 0, msg);
    return 0;
}

// ---- arithmetic ----

static const char* any_op_sym(int op) {
    switch (op) {
        case 0: return "+";
        case 1: return "-";
        case 2: return "*";
        case 3: return "/";
        case 4: return "//";
        case 5: return "%";
        case 6: return "**";
        case 7: return "<<";
        case 8: return ">>";
        case 9: return "&";
        case 10: return "|";
        default: return "^";
    }
}

static void* any_list_concat(void* p, void* q, int64_t tag) {
    int64_t ct = tag >> 8;
    int c = (int)(ct & 0xFF);
    int64_t w = (c == 'b') ? 1 : 8;
    int64_t ln = any_len_raw(p) + any_len_raw(q);
    int64_t cap = ln < 8 ? 8 : ln;
    void* r = malloc(32);
    if (!r) still_panic("memory", 0, "out of memory");
    ((still_hdr*)r)->rc = 1;
    ((still_hdr*)r)->destroy = ((still_hdr*)p)->destroy;
    *(int64_t*)((char*)r + 16) = 0;
    *(int64_t*)((char*)r + 24) = cap;
    void* data = malloc((size_t)cap * (size_t)w);
    if (!data) still_panic("memory", 0, "out of memory");
    *(void**)((char*)r + 32) = data;
    memcpy(data, any_list_data(p), (size_t)(any_len_raw(p) * w));
    memcpy((char*)data + any_len_raw(p) * w, any_list_data(q), (size_t)(any_len_raw(q) * w));
    *(int64_t*)((char*)r + 16) = ln;
    if (c == 's' || c == 'l' || c == 'd' || c == 'a') {
        for (int64_t i = 0; i < ln; i++) still_retain(*(void**)((char*)data + i * 8));
    }
    return r;
}

still_any* still_any_binop(int op, still_any* a, still_any* b) {
    if (!a || !b || (a->tag & 0xFF) == 'n' || (b->tag & 0xFF) == 'n') {
        still_panic("still", 0, "TypeError: cannot use none in arithmetic");
    }
    int ca = (int)(a->tag & 0xFF), cb = (int)(b->tag & 0xFF);
    int numa = (ca == 'i' || ca == 'b' || ca == 'f');
    int numb = (cb == 'i' || cb == 'b' || cb == 'f');
    if (numa && numb) {
        if (ca == 'f' || cb == 'f') {
            double x = any_num_f(a), y = any_num_f(b);
            switch (op) {
                case 0: return still_any_new_f(x + y);
                case 1: return still_any_new_f(x - y);
                case 2: return still_any_new_f(x * y);
                case 3: return still_any_new_f(x / y);
                case 4: return still_any_new_f(still_floordivf(x, y));
                case 5: return still_any_new_f(still_fmodf_(x, y));
                case 6: return still_any_new_f(pow(x, y));
                default: still_panic("still", 0, "TypeError: bitwise operations need whole numbers");
            }
        }
        int64_t x = a->u.i, y = b->u.i;
        switch (op) {
            case 0: return still_any_new_i(still_add(x, y, "still", 0));
            case 1: return still_any_new_i(still_sub(x, y, "still", 0));
            case 2: return still_any_new_i(still_mul(x, y, "still", 0));
            case 3: return still_any_new_f((double)x / (double)y);
            case 4: return still_any_new_i(still_floordiv(x, y, "still", 0));
            case 5: return still_any_new_i(still_imod(x, y, "still", 0));
            case 6: return still_any_new_i(still_ipow(x, y, "still", 0));
            case 7: return still_any_new_i(x << y);
            case 8: return still_any_new_i(x >> y);
            case 9: return still_any_new_i(x & y);
            case 10: return still_any_new_i(x | y);
            default: return still_any_new_i(x ^ y);
        }
    }
    if (ca == 's' && cb == 's' && op == 0) {
        return still_any_new_s(still_str_cat((still_str*)a->u.p, (still_str*)b->u.p));
    }
    if (ca == 'l' && cb == 'l' && a->tag == b->tag && op == 0) {
        return still_any_new_p(any_list_concat(a->u.p, b->u.p, a->tag), a->tag);
    }
    char msg[160];
    snprintf(msg, sizeof(msg), "TypeError: unsupported operand type(s) for %s: '%s' and '%s'",
             any_op_sym(op), still_any_typename(a), still_any_typename(b));
    still_panic("still", 0, msg);
    return NULL;
}

// ---- indexing / membership ----

// build an OWNED any cell holding element i of (base, stride, off) typed by ctag
static still_any* any_child_own(int64_t ctag, void* base, int64_t stride, int64_t off, int64_t i) {
    char* p = (char*)base + i * stride + off;
    int c = (int)(ctag & 0xFF);
    if (c == 'i') return still_any_new_i(*(int64_t*)p);
    if (c == 'f') return still_any_new_f(*(double*)p);
    if (c == 'b') return still_any_new_b((int64_t)(*(unsigned char*)p));
    if (c == 's') return still_any_new_s(*(still_str**)p);
    if (c == 'a') {
        still_any* x = *(still_any**)p;
        if (!x) return still_any_none();
        return (still_any*)still_retain(x);
    }
    if (c == 'l' || c == 'd') return still_any_new_p(*(void**)p, ctag);
    return still_any_none();
}

// compare raw element i of (base, stride, off) against an any cell
static int any_child_eq_cell(int64_t ctag, void* base, int64_t stride, int64_t off, int64_t i, still_any* item) {
    char* p = (char*)base + i * stride + off;
    int c = (int)(ctag & 0xFF);
    if (c == 'a') return still_any_eq(*(still_any**)p, item);
    still_any t;
    t.hdr.rc = 0;
    t.hdr.destroy = NULL;
    if (c == 'i') { t.tag = 'i'; t.u.i = *(int64_t*)p; }
    else if (c == 'f') { t.tag = 'f'; t.u.f = *(double*)p; }
    else if (c == 'b') { t.tag = 'b'; t.u.i = (int64_t)(*(unsigned char*)p != 0); }
    else if (c == 's') { t.tag = 's'; t.u.p = *(void**)p; }
    else if (c == 'l' || c == 'd') { t.tag = ctag; t.u.p = *(void**)p; }
    else return 0;
    return still_any_eq(&t, item);
}

static int64_t any_dict_find_cell(int64_t tag, void* d, still_any* key, int64_t* stride, int64_t* koff, int64_t* voff) {
    any_dict_geo(tag, stride, koff, voff);
    void* slots = any_dict_slots(d);
    int64_t kt = tag >> 8;
    for (int64_t i = 0; i < any_dict_cap(d); i++) {
        if (*((unsigned char*)slots + i * *stride) != 1) continue;
        if (any_child_eq_cell(kt, slots, *stride, *koff, i, key)) return i;
    }
    return -1;
}

still_any* still_any_index_i(still_any* a, int64_t i) {
    if (!a) still_panic("still", 0, "TypeError: cannot index none");
    int c = (int)(a->tag & 0xFF);
    if (c == 'l') {
        void* p = a->u.p;
        int64_t ct = a->tag >> 8;
        int64_t n = any_len_raw(p);
        if (i < 0) i += n;
        if (i < 0 || i >= n) still_panic("still", 0, "index out of range");
        return any_child_own(ct, any_list_data(p), any_stride_of(ct), 0, i);
    }
    if (c == 'd') {
        still_any k;
        k.hdr.rc = 0;
        k.hdr.destroy = NULL;
        k.tag = 'i';
        k.u.i = i;
        int64_t stride, koff, voff;
        int64_t j = any_dict_find_cell(a->tag, a->u.p, &k, &stride, &koff, &voff);
        if (j < 0) still_panic("still", 0, "missing key in dict");
        int64_t ks = any_tag_size(a->tag >> 8);
        int64_t vt = a->tag >> (8 * (1 + ks));
        return any_child_own(vt, any_dict_slots(a->u.p), stride, voff, j);
    }
    if (c == 's') {
        return still_any_new_s(still_str_index((still_str*)a->u.p, i, "still", 0));
    }
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: '%s' object is not subscriptable", still_any_typename(a));
    still_panic("still", 0, msg);
    return NULL;
}

still_any* still_any_index_s(still_any* a, still_str* k) {
    if (!a) still_panic("still", 0, "TypeError: cannot index none");
    int c = (int)(a->tag & 0xFF);
    if (c == 'd') {
        still_any k2;
        k2.hdr.rc = 0;
        k2.hdr.destroy = NULL;
        k2.tag = 's';
        k2.u.p = k;
        int64_t stride, koff, voff;
        int64_t j = any_dict_find_cell(a->tag, a->u.p, &k2, &stride, &koff, &voff);
        if (j < 0) still_panic("still", 0, "missing key in dict");
        int64_t ks = any_tag_size(a->tag >> 8);
        int64_t vt = a->tag >> (8 * (1 + ks));
        return any_child_own(vt, any_dict_slots(a->u.p), stride, voff, j);
    }
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: '%s' indices must be integers", still_any_typename(a));
    still_panic("still", 0, msg);
    return NULL;
}

int still_any_in(still_any* item, still_any* cont) {
    if (!item || !cont) still_panic("still", 0, "TypeError: argument of type 'none' is not iterable");
    int c = (int)(cont->tag & 0xFF);
    if (c == 's') {
        if ((item->tag & 0xFF) != 's') {
            still_panic("still", 0, "TypeError: 'in <string>' requires string as left operand");
        }
        return still_str_contains((still_str*)cont->u.p, (still_str*)item->u.p);
    }
    if (c == 'l') {
        int64_t ct = cont->tag >> 8;
        void* data = any_list_data(cont->u.p);
        int64_t st = any_stride_of(ct);
        for (int64_t i = 0; i < any_len_raw(cont->u.p); i++) {
            if (any_child_eq_cell(ct, data, st, 0, i, item)) return 1;
        }
        return 0;
    }
    if (c == 'd') {
        int64_t stride, koff, voff;
        return any_dict_find_cell(cont->tag, cont->u.p, item, &stride, &koff, &voff) >= 0;
    }
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: argument of type '%s' is not iterable", still_any_typename(cont));
    still_panic("still", 0, msg);
    return 0;
}

// membership against a *typed* container (list/dict/str) with an any key/item
int still_any_in_typed(int64_t ctag, void* cont, still_any* item) {
    if (!cont) still_panic("still", 0, "TypeError: argument of type 'none' is not iterable");
    int c = (int)(ctag & 0xFF);
    if (c == 's') {
        if (!item || (item->tag & 0xFF) != 's') {
            still_panic("still", 0, "TypeError: 'in <string>' requires string as left operand");
        }
        return still_str_contains((still_str*)cont, (still_str*)item->u.p);
    }
    if (c == 'l') {
        int64_t ct = ctag >> 8;
        void* data = any_list_data(cont);
        int64_t st = any_stride_of(ct);
        for (int64_t i = 0; i < any_len_raw(cont); i++) {
            if (any_child_eq_cell(ct, data, st, 0, i, item)) return 1;
        }
        return 0;
    }
    if (c == 'd') {
        int64_t stride, koff, voff;
        return any_dict_find_cell(ctag, cont, item, &stride, &koff, &voff) >= 0;
    }
    still_panic("still", 0, "TypeError: argument of type is not iterable");
    return 0;
}

// ---- length / casts / names ----

int64_t still_any_len(still_any* a) {
    if (!a) still_panic("still", 0, "TypeError: object of type 'none' has no len()");
    int c = (int)(a->tag & 0xFF);
    if (c == 's') return ((still_str*)a->u.p)->len;
    if (c == 'l' || c == 'd') return any_len_raw(a->u.p);
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: object of type '%s' has no len()", still_any_typename(a));
    still_panic("still", 0, msg);
    return 0;
}

int64_t still_any_to_int(still_any* a, const char* file, int64_t line) {
    if (!a || (a->tag & 0xFF) == 'n') still_panic(file, line, "TypeError: int() argument must not be none");
    int c = (int)(a->tag & 0xFF);
    if (c == 'i' || c == 'b') return a->u.i;
    if (c == 'f') return (int64_t)a->u.f;
    if (c == 's') return still_str_to_int((still_str*)a->u.p, file, line);
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: int() argument must be int, float, str or bool, not '%s'", still_any_typename(a));
    still_panic(file, line, msg);
    return 0;
}

double still_any_to_float(still_any* a, const char* file, int64_t line) {
    if (!a || (a->tag & 0xFF) == 'n') still_panic(file, line, "TypeError: float() argument must not be none");
    int c = (int)(a->tag & 0xFF);
    if (c == 'f') return a->u.f;
    if (c == 'i' || c == 'b') return (double)a->u.i;
    if (c == 's') return still_str_to_float((still_str*)a->u.p, file, line);
    char msg[128];
    snprintf(msg, sizeof(msg), "TypeError: float() argument must be int, float, str or bool, not '%s'", still_any_typename(a));
    still_panic(file, line, msg);
    return 0;
}

const char* still_any_typename(still_any* a) {
    if (!a) return "none";
    switch (a->tag & 0xFF) {
        case 'i': return "int";
        case 'f': return "float";
        case 'b': return "bool";
        case 's': return "str";
        case 'l': return "list";
        case 'd': return "dict";
    }
    return "none";
}

// ---- printing / string conversion ----

static void any_tostr_a(still_sb* sb, still_any* a, int quote);

static void any_tostr_child(still_sb* sb, int64_t ctag, void* base, int64_t stride, int64_t off, int64_t i) {
    char* p = (char*)base + i * stride + off;
    int c = (int)(ctag & 0xFF);
    if (c == 'i') { still_sb_add_int(sb, *(int64_t*)p); return; }
    if (c == 'f') { still_sb_add_float(sb, *(double*)p); return; }
    if (c == 'b') { still_sb_add_bool(sb, *(unsigned char*)p != 0); return; }
    if (c == 's') {
        still_sb_c(sb, '"');
        still_sb_s(sb, *(still_str**)p);
        still_sb_c(sb, '"');
        return;
    }
    if (c == 'a') {
        any_tostr_a(sb, *(still_any**)p, 1);
        return;
    }
    if (c == 'l' || c == 'd') {
        still_any t;
        t.hdr.rc = 0;
        t.hdr.destroy = NULL;
        t.tag = ctag;
        t.u.p = *(void**)p;
        any_tostr_a(sb, &t, 1);
        return;
    }
    still_sb_raw(sb, "none");
}

static void any_tostr_a(still_sb* sb, still_any* a, int quote) {
    if (!a || (a->tag & 0xFF) == 'n') {
        still_sb_raw(sb, "none");
        return;
    }
    switch (a->tag & 0xFF) {
        case 'i':
            still_sb_add_int(sb, a->u.i);
            return;
        case 'f':
            still_sb_add_float(sb, a->u.f);
            return;
        case 'b':
            still_sb_add_bool(sb, a->u.i != 0);
            return;
        case 's':
            if (quote) still_sb_c(sb, '"');
            still_sb_s(sb, (still_str*)a->u.p);
            if (quote) still_sb_c(sb, '"');
            return;
        case 'l': {
            int64_t ct = a->tag >> 8;
            void* p = a->u.p;
            void* data = any_list_data(p);
            int64_t st = any_stride_of(ct);
            still_sb_c(sb, '[');
            for (int64_t i = 0; i < any_len_raw(p); i++) {
                if (i) still_sb_raw(sb, ", ");
                any_tostr_child(sb, ct, data, st, 0, i);
            }
            still_sb_c(sb, ']');
            return;
        }
        case 'd': {
            int64_t ks = any_tag_size(a->tag >> 8);
            int64_t kt = a->tag >> 8;
            int64_t vt = a->tag >> (8 * (1 + ks));
            int64_t stride, koff, voff;
            any_dict_geo(a->tag, &stride, &koff, &voff);
            void* slots = any_dict_slots(a->u.p);
            still_sb_c(sb, '{');
            int first = 1;
            for (int64_t i = 0; i < any_dict_cap(a->u.p); i++) {
                if (*((unsigned char*)slots + i * stride) != 1) continue;
                if (!first) still_sb_raw(sb, ", ");
                first = 0;
                any_tostr_child(sb, kt, slots, stride, koff, i);
                still_sb_raw(sb, ": ");
                any_tostr_child(sb, vt, slots, stride, voff, i);
            }
            still_sb_c(sb, '}');
            return;
        }
    }
}

void still_tostr_add_a(still_sb* sb, still_any* a) {
    any_tostr_a(sb, a, 0);
}

still_str* still_any_tostr(still_any* a) {
    still_sb* sb = still_sb_new();
    any_tostr_a(sb, a, 0);
    return still_sb_finish(sb);
}

still_str* still_any_repr(still_any* a) {
    still_sb* sb = still_sb_new();
    any_tostr_a(sb, a, 1);
    return still_sb_finish(sb);
}

void still_print_any(still_any* a) {
    still_str* s = still_any_tostr(a);
    still_print_str(s);
    still_release(s);
}

// ---- hashing ----

static uint64_t any_hash_f(double f) {
    if (f == (double)(int64_t)f && f >= -9.2e18 && f <= 9.2e18) return still_hash_i64((int64_t)f);
    int64_t bits;
    memcpy(&bits, &f, 8);
    return still_hash_i64(bits);
}

static uint64_t any_hash_child(int64_t ctag, void* base, int64_t stride, int64_t off, int64_t i) {
    char* p = (char*)base + i * stride + off;
    int c = (int)(ctag & 0xFF);
    if (c == 'i') return still_hash_i64(*(int64_t*)p);
    if (c == 'b') return still_hash_i64((int64_t)(*(unsigned char*)p != 0));
    if (c == 'f') return any_hash_f(*(double*)p);
    if (c == 's') return still_hash_str(*(still_str**)p);
    if (c == 'a') {
        still_any* x = *(still_any**)p;
        return x ? still_hash_a(x) : 0;
    }
    if (c == 'l' || c == 'd') return any_hash_raw(ctag, *(void**)p);
    return 0;
}

static uint64_t any_hash_raw(int64_t tag, void* p) {
    if (!p) return 0;
    int c = (int)(tag & 0xFF);
    if (c == 'l') {
        int64_t ct = tag >> 8;
        uint64_t h = still_hash_i64(any_len_raw(p));
        void* data = any_list_data(p);
        int64_t st = any_stride_of(ct);
        for (int64_t i = 0; i < any_len_raw(p); i++) {
            h = h * 1000003ULL + any_hash_child(ct, data, st, 0, i);
        }
        return h;
    }
    if (c == 'd') {
        int64_t ks = any_tag_size(tag >> 8);
        int64_t kt = tag >> 8;
        int64_t vt = tag >> (8 * (1 + ks));
        int64_t stride, koff, voff;
        any_dict_geo(tag, &stride, &koff, &voff);
        void* slots = any_dict_slots(p);
        uint64_t acc = 0;
        for (int64_t i = 0; i < any_dict_cap(p); i++) {
            if (*((unsigned char*)slots + i * stride) != 1) continue;
            uint64_t kh = any_hash_child(kt, slots, stride, koff, i);
            uint64_t vh = any_hash_child(vt, slots, stride, voff, i);
            acc += kh ^ (vh * 0x9E3779B97F4A7C15ULL);
        }
        return acc;
    }
    return 0;
}

uint64_t still_hash_a(still_any* a) {
    if (!a || (a->tag & 0xFF) == 'n') return 0;
    switch (a->tag & 0xFF) {
        case 'i': case 'b': return still_hash_i64(a->u.i);
        case 'f': return any_hash_f(a->u.f);
        case 's': return still_hash_str((still_str*)a->u.p);
        case 'l': case 'd': return any_hash_raw(a->tag, a->u.p);
    }
    return 0;
}

// ---- in-place string append (linear `s += t`) ----

void still_str_append_inplace(still_str** slot, still_str* add) {
    still_str* s = *slot;
    if (!s) {
        *slot = (still_str*)still_retain(add);
        return;
    }
    if (s == add) {
        *slot = still_str_cat(s, s);
        still_release(s);
        return;
    }
    if (s->hdr.rc == 1) {
        int64_t n = s->len + add->len;
        if (n > s->cap) {
            int64_t nc = s->cap > 0 ? s->cap : 8;
            while (nc < n + 1) nc *= 2;
            still_str* ns = (still_str*)realloc(s, sizeof(still_str) + (size_t)nc + 1);
            if (!ns) still_panic("memory", 0, "out of memory");
            ns->cap = nc;
            s = ns;
            *slot = ns;
        }
        memcpy(s->bytes + s->len, add->bytes, (size_t)add->len);
        s->len = n;
        s->bytes[n] = 0;
        return;
    }
    *slot = still_str_cat(s, add);
    still_release(s);
}
