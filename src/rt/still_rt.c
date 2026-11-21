#include "still_rt.h"
#include <math.h>
#include <time.h>
#include <errno.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <signal.h>
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
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

void* still_unwrap_none_panic(const char* file, int64_t line) {
    still_panic(file, line, "unwrapped a none value");
    return 0; /* unreachable */
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
    /* GUI-subsystem programs have no console; when stdout is redirected to
       a file (CI probes) it is fully buffered and would be lost on a crash
       — flush per line like a terminal does */
    fflush(stdout);
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

/* Match an env entry against the reported __FILE__: exact, or the report
 * ends with the entry at a path boundary (the compiler prefixes the build
 * directory to #line file names, and basename entries must not match
 * longer names like "xpanic_line.strc"). */
static int still_bp_file_eq(const char* envf, const char* file) {
    if (strcmp(envf, file) == 0) return 1;
    size_t elen = strlen(envf);
    size_t flen = strlen(file);
    if (elen == 0 || elen > flen) return 0;
    const char* tail = file + (flen - elen);
    if (strcmp(tail, envf) != 0) return 0;
    if (tail == file) return 1;
    char prev = tail[-1];
    return prev == '/' || prev == '\\';
}

void still_bp_check(const char* file, int64_t line) {
    if (!still_bp_any) return;
    for (int i = 0; i < still_bp_count; i++) {
        if (still_bp_lines[i] == line && still_bp_file_eq(still_bp_files[i], file)) {
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

// Forward declarations for the mutually recursive equality/ordering/hashing
// walkers below (container forms recurse back through the cell comparators).
// C99 has no implicit function declarations; strict gcc and MSVC reject
// use-before-define, so these must exist even though tcc tolerates it.
static int any_raw_eq(int64_t tag, void* p, void* q);
static int any_raw_cmp(int64_t tag, void* p, void* q);
static uint64_t any_hash_raw(int64_t tag, void* p);

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








/* ---------------------------------------------------------------- Universal platform modules */
/* net / gui / audio / media, added in Still v0.6.0.

   Design follows the Python-bridge pattern: system libraries are loaded at run
   time via LoadLibraryA/GetProcAddress, so programs that never touch these
   modules carry zero extra link-time dependencies. user32/gdi32 remain directly
   linked (the compiler already links them by default); winhttp, winmm and
   gdiplus are resolved lazily here.

   Every entry point degrades gracefully when the backend is unavailable (for
   example running the same program on Linux): net_request returns -1, gui
   functions return 0/1-neutral values, audio returns non-zero, media returns 0.
   Callers in the stdlib surface this as net_available(), gui_available(), ...
   so a program can branch without crashing. */

/* UTF-8 <-> wide helpers (kernel32, always available on Windows) */
/* Universal any-dict helper (both platforms): return the i-th key of a
   string-keyed dict held in an `any` as a retained still_str (NULL-compatible
   empty string when i is out of range or the value is not a string-keyed
   dict). Lets Still programs iterate dict keys carried in `any` without
   panicking (pairs with len()). Builtin convention: returns a retained
   still_str* the caller releases. */
still_str* still_dict_key_at(still_any* a, int64_t i) {
    if (!a) return still_str_new("", 0);
    int c = (int)(a->tag & 0xFF);
    if (c != 'd') return still_str_new("", 0);
    int kt = (int)((a->tag >> 8) & 0xFF);
    if (kt != 's') return still_str_new("", 0);
    int64_t stride, koff, voff;
    any_dict_geo(a->tag, &stride, &koff, &voff);
    void* slots = any_dict_slots(a->u.p);
    int64_t used_seen = 0;
    for (int64_t j = 0; j < any_dict_cap(a->u.p); j++) {
        if (*((unsigned char*)slots + j * stride) != 1) continue;
        if (used_seen == i) {
            still_str* k = *(still_str**)((char*)slots + j * stride + koff);
            if (!k) return still_str_new("", 0);
            return still_str_new(k->bytes, k->len);
        }
        used_seen++;
    }
    return still_str_new("", 0);
}

#ifdef _WIN32
static wchar_t* still_wide(const char* s) {
    int n = MultiByteToWideChar(CP_UTF8, 0, s, -1, NULL, 0);
    wchar_t* w = (wchar_t*)malloc(sizeof(wchar_t) * (size_t)(n > 0 ? n : 1));
    if (!w) still_panic("memory", 0, "out of memory");
    if (n > 0) MultiByteToWideChar(CP_UTF8, 0, s, -1, w, n);
    return w;
}

static char* still_narrow(const wchar_t* w) {
    int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, NULL, 0, NULL, NULL);
    char* s = (char*)malloc((size_t)(n > 0 ? n : 1));
    if (!s) still_panic("memory", 0, "out of memory");
    if (n > 0) WideCharToMultiByte(CP_UTF8, 0, w, -1, s, n, NULL, NULL);
    return s;
}

static char* still_dup_cstr(const char* s) {
    size_t n = strlen(s);
    char* p = (char*)malloc(n + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, s, n + 1);
    return p;
}

/* ============================================================ net: HTTPS/HTTP client */

/* winhttp.h constants (we declare the functions ourselves and do not include
   winhttp.h, so mirror the values needed here) */
#define STIL_WINHTTP_ACCESS_TYPE_DEFAULT_PROXY 0
#define STIL_WINHTTP_FLAG_SECURE 0x00800000
#define STIL_WINHTTP_QUERY_STATUS_CODE 19
#define STIL_WINHTTP_QUERY_RAW_HEADERS_CRLF 22
#define STIL_WINHTTP_QUERY_CONTENT_TYPE 1
#define STIL_WINHTTP_QUERY_FLAG_NUMBER 0x20000000
#define STIL_WINHTTP_OPTION_CONNECT_TIMEOUT 3
#define STIL_WINHTTP_OPTION_SEND_TIMEOUT 5
#define STIL_WINHTTP_OPTION_RECEIVE_TIMEOUT 6
#define STIL_WINHTTP_OPTION_DECOMPRESSION 118
#define STIL_WINHTTP_DECOMPRESSION_FLAG_GZIP 0x00000001
#define STIL_WINHTTP_DECOMPRESSION_FLAG_DEFLATE 0x00000002
#define STIL_WINHTTP_ADDREQ_FLAG_ADD 0x20000000
#define STIL_WINHTTP_ADDREQ_FLAG_REPLACE 0x80000000

#define STIL_NET_SLOTS 16
#define STIL_NET_MAX_BODY (64 * 1024 * 1024)

typedef struct {
    int used;
    int64_t status;
    char* raw_headers;   /* NUL-terminated, \r\n separated */
    char* body;          /* NUL-terminated text body */
    size_t body_len;
} StilNetSlot;

static StilNetSlot g_net_slots[STIL_NET_SLOTS];
static char g_net_err[512] = "";
static int g_net_slots_inited = 0;
static HMODULE g_net_winhttp = NULL;

static void net_set_err(const char* msg) {
    snprintf(g_net_err, sizeof(g_net_err), "%s", msg);
}

static void net_init_slots(void) {
    if (g_net_slots_inited) return;
    for (int i = 0; i < STIL_NET_SLOTS; i++) {
        g_net_slots[i].used = 0;
        g_net_slots[i].status = 0;
        g_net_slots[i].raw_headers = NULL;
        g_net_slots[i].body = NULL;
        g_net_slots[i].body_len = 0;
    }
    g_net_slots_inited = 1;
}

static int g_net_trace = -1;
static void net_trace(const char* stage) {
    if (g_net_trace < 0) g_net_trace = (getenv("STILL_NET_TRACE") != NULL) ? 1 : 0;
    if (g_net_trace) fprintf(stderr, "still_net: %s\n", stage);
}

static int net_load_winhttp(void) {
    if (g_net_winhttp) return 1;
    g_net_winhttp = LoadLibraryA("winhttp.dll");
    if (!g_net_winhttp) {
        net_set_err("cannot load winhttp.dll (network backend requires Windows 7 or later)");
        return 0;
    }
    return 1;
}

int64_t still_net_available(void) {
    return net_load_winhttp() ? 1 : 0;
}

const char* still_net_last_error(void) {
    return g_net_err[0] ? g_net_err : "no error";
}

static void net_slot_free(int id) {
    if (id < 0 || id >= STIL_NET_SLOTS) return;
    StilNetSlot* s = &g_net_slots[id];
    free(s->raw_headers);
    free(s->body);
    s->raw_headers = NULL;
    s->body = NULL;
    s->body_len = 0;
    s->status = 0;
    s->used = 0;
}

int64_t still_net_request(const char* method, const char* url, const char* headers,
                          const char* body, int64_t timeout_ms) {
    net_init_slots();
    net_trace("request enter");
    if (!net_load_winhttp()) return -1;
    net_trace("winhttp loaded");

    typedef void* (WINAPI *OpenFn)(const wchar_t*, DWORD, const wchar_t*, const wchar_t*, DWORD);
    typedef void* (WINAPI *ConnectFn)(void*, const wchar_t*, unsigned short, DWORD);
    typedef void* (WINAPI *OpenReqFn)(void*, const wchar_t*, const wchar_t*, const wchar_t*, const wchar_t*, const wchar_t**, DWORD);
    typedef BOOL (WINAPI *AddHdrFn)(void*, const wchar_t*, DWORD, DWORD);
    typedef BOOL (WINAPI *SendFn)(void*, const wchar_t*, DWORD, LPVOID, DWORD, DWORD, DWORD_PTR);
    typedef BOOL (WINAPI *RecvFn)(void*, LPVOID, DWORD, DWORD);
    typedef BOOL (WINAPI *QueryDataFn)(void*, LPDWORD);
    typedef BOOL (WINAPI *ReadFn)(void*, LPVOID, DWORD, LPDWORD);
    typedef BOOL (WINAPI *QueryHdrFn)(void*, DWORD, const wchar_t*, LPVOID, LPDWORD, LPDWORD);
    typedef BOOL (WINAPI *SetOptFn)(void*, DWORD, LPVOID, DWORD);
    typedef BOOL (WINAPI *CloseFn)(void*);

    OpenFn wOpen = (OpenFn)GetProcAddress(g_net_winhttp, "WinHttpOpen");
    ConnectFn wConnect = (ConnectFn)GetProcAddress(g_net_winhttp, "WinHttpConnect");
    OpenReqFn wOpenReq = (OpenReqFn)GetProcAddress(g_net_winhttp, "WinHttpOpenRequest");
    AddHdrFn wAddHdr = (AddHdrFn)GetProcAddress(g_net_winhttp, "WinHttpAddRequestHeaders");
    SendFn wSend = (SendFn)GetProcAddress(g_net_winhttp, "WinHttpSendRequest");
    RecvFn wRecv = (RecvFn)GetProcAddress(g_net_winhttp, "WinHttpReceiveResponse");
    QueryDataFn wQueryData = (QueryDataFn)GetProcAddress(g_net_winhttp, "WinHttpQueryDataAvailable");
    ReadFn wRead = (ReadFn)GetProcAddress(g_net_winhttp, "WinHttpReadData");
    QueryHdrFn wQueryHdr = (QueryHdrFn)GetProcAddress(g_net_winhttp, "WinHttpQueryHeaders");
    SetOptFn wSetOpt = (SetOptFn)GetProcAddress(g_net_winhttp, "WinHttpSetOption");
    CloseFn wClose = (CloseFn)GetProcAddress(g_net_winhttp, "WinHttpCloseHandle");
    if (!wOpen || !wConnect || !wOpenReq || !wAddHdr || !wSend || !wRecv || !wQueryData || !wRead || !wQueryHdr || !wClose) {
        net_set_err("winhttp.dll is missing required exports");
        return -1;
    }

    /* --- parse URL: scheme://host[:port]/path --- */
    const char* rest = strstr(url, "://");
    int secure = 0;
    const char* host_start;
    if (rest) {
        secure = (strncmp(url, "https", 5) == 0);
        host_start = rest + 3;
    } else {
        host_start = url;
    }
    const char* path_start = strchr(host_start, '/');
    const char* port_start = NULL;
    const char* scan = host_start;
    while (scan && *scan) {
        if (*scan == '/') break;
        if (*scan == ':') { port_start = scan + 1; break; }
        scan++;
    }
    size_t host_len = (port_start ? (size_t)(port_start - host_start)
                                  : (path_start ? (size_t)(path_start - host_start) : strlen(host_start)));
    if (host_len == 0 || host_len > 256) {
        net_set_err("invalid URL host");
        return -1;
    }
    char host[257];
    memcpy(host, host_start, host_len);
    host[host_len] = 0;
    int port = secure ? 443 : 80;
    if (port_start) {
        const char* pe = port_start;
        while (*pe >= '0' && *pe <= '9') pe++;
        if (pe > port_start) port = atoi(port_start);
        if (!path_start || pe > path_start) path_start = pe;
    }
    if (!path_start) path_start = "/";
    if (timeout_ms < 1) timeout_ms = 15000;
    if (timeout_ms > 120000) timeout_ms = 120000;

    wchar_t* w_host = still_wide(host);
    wchar_t* w_path = still_wide(path_start);
    wchar_t* w_method = still_wide(method);
    if (!w_host || !w_path || !w_method) {
        free(w_host); free(w_path); free(w_method);
        net_set_err("out of memory");
        return -1;
    }

    net_trace("opening session");
    void* hSession = wOpen(L"Still/0.6", STIL_WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, NULL, NULL, 0);
    void* hConnect = NULL;
    void* hRequest = NULL;
    int slot_id = -1;
    char werr[512];

    do {
        if (!hSession) { net_set_err("WinHttpOpen failed"); break; }
        net_trace("connecting");
        hConnect = wConnect(hSession, w_host, (unsigned short)port, 0);
        if (!hConnect) { snprintf(werr, sizeof(werr), "cannot connect to %s:%d", host, port); net_set_err(werr); break; }
        net_trace("opening request");
        hRequest = wOpenReq(hConnect, w_method, w_path, NULL, NULL, NULL,
                            secure ? STIL_WINHTTP_FLAG_SECURE : 0);
        if (!hRequest) { net_set_err("WinHttpOpenRequest failed"); break; }
        net_trace("setting options + headers");

        if (wSetOpt) {
            DWORD t = (DWORD)timeout_ms;
            wSetOpt(hRequest, STIL_WINHTTP_OPTION_CONNECT_TIMEOUT, &t, sizeof(t));
            wSetOpt(hRequest, STIL_WINHTTP_OPTION_SEND_TIMEOUT, &t, sizeof(t));
            wSetOpt(hRequest, STIL_WINHTTP_OPTION_RECEIVE_TIMEOUT, &t, sizeof(t));
            /* auto-decompress gzip/deflate responses (Windows 8.1+) */
            DWORD decomp = STIL_WINHTTP_DECOMPRESSION_FLAG_GZIP | STIL_WINHTTP_DECOMPRESSION_FLAG_DEFLATE;
            wSetOpt(hRequest, STIL_WINHTTP_OPTION_DECOMPRESSION, &decomp, sizeof(decomp));
        }

        if (headers && headers[0]) {
            /* duplicate the header block so we can turn \n into \r\n */
            size_t hl = strlen(headers);
            char* hcopy = (char*)malloc(hl + 2);
            if (!hcopy) { net_set_err("out of memory"); break; }
            memcpy(hcopy, headers, hl + 1);
            for (size_t i = 0; i < hl; i++) {
                if (hcopy[i] == '\n' && (i == 0 || hcopy[i - 1] != '\r')) {
                    memmove(hcopy + i + 1, hcopy + i, hl - i + 1);
                    hcopy[i] = '\r';
                    hl++;
                    i++;
                }
            }
            wchar_t* w_hdrs = still_wide(hcopy);
            free(hcopy);
            if (w_hdrs) {
                wAddHdr(hRequest, w_hdrs, (DWORD)-1,
                        STIL_WINHTTP_ADDREQ_FLAG_ADD | STIL_WINHTTP_ADDREQ_FLAG_REPLACE);
                free(w_hdrs);
            }
        }

        net_trace("sending");
        size_t body_len = body ? strlen(body) : 0;
        BOOL sent = wSend(hRequest, NULL, 0,
                          (LPVOID)(body ? body : ""), (DWORD)body_len,
                          (DWORD)body_len, 0);
        if (!sent) { net_set_err("request send failed (offline, DNS, TLS or timeout)"); break; }
        net_trace("sent; receiving response");
        if (!wRecv(hRequest, NULL, 0, 0)) { net_set_err("no response received"); break; }
        net_trace("response received; reading status");

        for (int i = 0; i < STIL_NET_SLOTS; i++) {
            if (!g_net_slots[i].used) { slot_id = i; break; }
        }
        if (slot_id < 0) { net_set_err("too many uncollected responses (call net_free)"); break; }
        StilNetSlot* slot = &g_net_slots[slot_id];

        DWORD status = 0, sz = sizeof(status);
        wQueryHdr(hRequest, STIL_WINHTTP_QUERY_STATUS_CODE | STIL_WINHTTP_QUERY_FLAG_NUMBER,
                  NULL, &status, &sz, NULL);
        slot->status = (int64_t)status;
        slot->used = 1;
        net_trace("status read; reading headers");

        /* content-type (queried per-header; avoids the raw-headers buffer) */
        char ctype[256] = "";
        DWORD ctype_size = sizeof(ctype);
        wQueryHdr(hRequest, STIL_WINHTTP_QUERY_CONTENT_TYPE,
                  NULL, (LPVOID)ctype, &ctype_size, NULL);
        if (ctype[0] != 0) {
            slot->raw_headers = still_dup_cstr("Content-Type: ");
            size_t clen = strlen(ctype);
            char* grown = (char*)realloc(slot->raw_headers, 15 + clen + 1);
            if (grown) {
                memcpy(grown + 14, ctype, clen + 1);
                slot->raw_headers = grown;
            }
        }

        /* body */
        DWORD avail = 0;
        size_t total = 0;
        char* acc = (char*)malloc(1);
        if (acc) acc[0] = 0;
        while (wQueryData(hRequest, &avail) && avail > 0) {
            if (total + (size_t)avail > (size_t)STIL_NET_MAX_BODY) break;
            char* nb = (char*)realloc(acc, total + (size_t)avail + 1);
            if (!nb) break;
            acc = nb;
            DWORD got = 0;
            if (!wRead(hRequest, acc + total, avail, &got) || got == 0) break;
            total += got;
            acc[total] = 0;
        }
        if (!acc) acc = still_dup_cstr("");
        slot->body = acc;
        slot->body_len = total;
        g_net_err[0] = 0;
        net_trace("body read complete");
    } while (0);

    if (hRequest) wClose(hRequest);
    if (hConnect) wClose(hConnect);
    if (hSession) wClose(hSession);
    free(w_host);
    free(w_path);
    free(w_method);

    if (slot_id < 0) { net_trace("request failed"); return -1; }
    net_trace("request ok");
    return slot_id;
}

int64_t still_net_status(int64_t id) {
    if (id < 0 || id >= STIL_NET_SLOTS || !g_net_slots[id].used) return 0;
    return g_net_slots[id].status;
}

static const char* net_find_header(const char* raw, const char* name) {
    if (!raw) return NULL;
    size_t nl = strlen(name);
    const char* p = raw;
    while (p && *p) {
        const char* eol = strstr(p, "\r\n");
        size_t line_len = eol ? (size_t)(eol - p) : strlen(p);
        const char* colon = (const char*)memchr(p, ':', line_len);
        if (colon) {
            size_t name_len = (size_t)(colon - p);
            if (name_len == nl) {
                int eq = 1;
                for (size_t i = 0; i < nl; i++) {
                    char a = p[i]; char b = name[i];
                    if (a >= 'A' && a <= 'Z') a = (char)(a + 32);
                    if (b >= 'A' && b <= 'Z') b = (char)(b + 32);
                    if (a != b) { eq = 0; break; }
                }
                if (eq) {
                    const char* v = colon + 1;
                    while (*v == ' ' || *v == '\t') v++;
                    return v;
                }
            }
        }
        if (!eol) break;
        p = eol + 2;
    }
    return NULL;
}

const char* still_net_header(int64_t id, const char* name) {
    if (id < 0 || id >= STIL_NET_SLOTS || !g_net_slots[id].used) return "";
    const char* v = net_find_header(g_net_slots[id].raw_headers, name);
    return v ? v : "";
}

const char* still_net_body(int64_t id) {
    if (id < 0 || id >= STIL_NET_SLOTS || !g_net_slots[id].used) return "";
    return g_net_slots[id].body ? g_net_slots[id].body : "";
}

/* Save a response body to disk byte-for-byte (binary safe: images, audio).
   Returns 1 on success, 0 on failure. The slot stays open until net.free. */
int64_t still_net_save_body(int64_t id, const char* path) {
    if (id < 0 || id >= STIL_NET_SLOTS || !g_net_slots[id].used || !path || !path[0]) return 0;
    if (!g_net_slots[id].body || g_net_slots[id].body_len == 0) return 0;
    FILE* f = fopen(path, "wb");
    if (!f) return 0;
    size_t n = fwrite(g_net_slots[id].body, 1, g_net_slots[id].body_len, f);
    fclose(f);
    return (n == g_net_slots[id].body_len) ? 1 : 0;
}

void still_net_free(int64_t id) {
    if (id < 0 || id >= STIL_NET_SLOTS) return;
    net_slot_free(id);
}

/* ============================================================ gui: Win32 window + GDI */

#define STIL_GUI_MAX 8
#define STIL_GUI_FONT_CACHE 32

typedef struct {
    double size;      /* positive = regular, negative = semibold */
    HFONT font;
} StilFontSlot;

#ifndef WM_DPICHANGED
#define WM_DPICHANGED 0x02E0
#endif

/* Minimal MINMAXINFO replica (avoids depending on SDK header versions) */
typedef struct {
    POINT ptReserved1;
    POINT ptReserved2;
    POINT ptMaxSize;
    POINT ptMaxPosition;
    POINT ptMinTrackSize;
    POINT ptMaxTrackSize;
} StilMinMaxInfo;

typedef struct {
    HWND hwnd;
    HDC mem;
    HBITMAP bmp;
    int w, h;
    int open;
    int bg;
    int mouse_x, mouse_y;   /* physical client pixels */
    int ldown;
    int click_edge;
    int wheel;              /* physical wheel units */
    char input[4096];
    int input_len;
    unsigned char keys[256];
    unsigned char keys_prev[256];
    unsigned char keys_edge[256];
    StilFontSlot fonts[STIL_GUI_FONT_CACHE];
    int font_count;
    int dpi;
    double scale;           /* logical->physical factor (dpi/96), 1.0 default */
} StilWindow;

static StilWindow g_wins[STIL_GUI_MAX];
static int g_gui_class_registered = 0;

LRESULT CALLBACK gui_wndproc_real(HWND, UINT, WPARAM, LPARAM);

static void gui_register_class(void) {
    if (g_gui_class_registered) return;
    WNDCLASSEXA wc;
    memset(&wc, 0, sizeof(wc));
    wc.cbSize = sizeof(wc);
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = gui_wndproc_real;
    wc.hCursor = LoadCursor(NULL, (LPCSTR)IDC_ARROW);
    wc.hbrBackground = NULL;
    wc.lpszClassName = "StillWindow";
    wc.hInstance = GetModuleHandleA(NULL);
    RegisterClassExA(&wc);
    g_gui_class_registered = 1;
}

LRESULT CALLBACK gui_wndproc_real(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    StilWindow* w = NULL;
    for (int i = 0; i < STIL_GUI_MAX; i++) {
        if (g_wins[i].hwnd == hwnd && hwnd) { w = &g_wins[i]; break; }
    }
    switch (msg) {
        case WM_CHAR: {
            if (w && w->input_len < (int)sizeof(w->input) - 1 && wp >= 32 && wp < 256) {
                w->input[w->input_len++] = (char)wp;
                w->input[w->input_len] = 0;
            }
            return 0;
        }
        case WM_KEYDOWN:
        case WM_SYSKEYDOWN: {
            if (w && wp < 256) w->keys[wp] = 1;
            break;
        }
        case WM_KEYUP:
        case WM_SYSKEYUP: {
            if (w && wp < 256) w->keys[wp] = 0;
            break;
        }
        case WM_MOUSEWHEEL: {
            if (w) w->wheel += (short)HIWORD(wp);
            return 0;
        }
        case WM_GETMINMAXINFO: {
            /* keep the window resizable/maximizable with a sane minimum size */
            StilMinMaxInfo* mmi = (StilMinMaxInfo*)lp;
            if (mmi) {
                UINT dpi = (w && w->dpi > 0) ? (UINT)w->dpi : 96;
                mmi->ptMinTrackSize.x = (LONG)(460 * dpi / 96);
                mmi->ptMinTrackSize.y = (LONG)(320 * dpi / 96);
                mmi->ptMaxTrackSize.x = GetSystemMetrics(SM_CXMAXTRACK);
                mmi->ptMaxTrackSize.y = GetSystemMetrics(SM_CYMAXTRACK);
            }
            return 0;
        }
        case WM_DPICHANGED: {
            if (w) w->dpi = (int)LOWORD(wp);
            RECT* sugg = (RECT*)lp;
            if (sugg) {
                SetWindowPos(hwnd, NULL, sugg->left, sugg->top,
                             sugg->right - sugg->left, sugg->bottom - sugg->top,
                             SWP_NOZORDER | SWP_NOACTIVATE);
            }
            return 0;
        }
        case WM_PAINT: {
            PAINTSTRUCT ps;
            BeginPaint(hwnd, &ps);
            EndPaint(hwnd, &ps);
            return 0;
        }
        case WM_ERASEBKGND:
            return 1;
        case WM_CLOSE: {
            if (w) w->open = 0;
            DestroyWindow(hwnd);
            return 0;
        }
        case WM_DESTROY: {
            if (w) w->open = 0;
            return 0;
        }
    }
    return DefWindowProcA(hwnd, msg, wp, lp);
}

static StilWindow* gui_win(int64_t handle) {
    if (!handle) return NULL;
    for (int i = 0; i < STIL_GUI_MAX; i++) {
        if (g_wins[i].hwnd == (HWND)(uintptr_t)handle) return &g_wins[i];
    }
    return NULL;
}

int64_t still_gui_available(void) {
    return 1;
}

static UINT gui_window_dpi(HWND hwnd) {
    typedef UINT (WINAPI *GetDpiForWindowFn)(HWND);
    GetDpiForWindowFn f = (GetDpiForWindowFn)GetProcAddress(GetModuleHandleA("user32.dll"), "GetDpiForWindow");
    if (f && hwnd) {
        UINT d = f(hwnd);
        if (d > 0) return d;
    }
    HDC hdc = GetDC(hwnd);
    UINT d = (UINT)GetDeviceCaps(hdc, LOGPIXELSX);
    ReleaseDC(hwnd, hdc);
    return d ? d : 96;
}

/* Per-monitor-v2 DPI awareness so coordinates are real pixels and text is
   crisp; falls back gracefully on older Windows. Must run before the first
   window is created. */
static void gui_enable_dpi_awareness(void) {
    typedef BOOL (WINAPI *SetCtxFn)(void*);
    SetCtxFn setctx = (SetCtxFn)GetProcAddress(GetModuleHandleA("user32.dll"), "SetProcessDpiAwarenessContext");
    if (setctx) {
        /* DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 == (HANDLE)-4 */
        if (setctx((void*)(intptr_t)-4)) return;
    }
    typedef BOOL (WINAPI *DpiAwareFn)(void);
    DpiAwareFn dpi = (DpiAwareFn)GetProcAddress(GetModuleHandleA("user32.dll"), "SetProcessDPIAware");
    if (dpi) dpi();
}

int64_t still_gui_open_window(const char* title, int64_t w, int64_t h) {
    if (w < 16) w = 16;
    if (h < 16) h = 16;
    if (w > 3840) w = 3840;
    if (h > 2160) h = 2160;
    gui_register_class();
    /* crisp text and real-pixel coordinates on scaled displays */
    gui_enable_dpi_awareness();

    int slot = -1;
    for (int i = 0; i < STIL_GUI_MAX; i++) {
        if (!g_wins[i].hwnd && !g_wins[i].open) { slot = i; break; }
    }
    if (slot < 0) return 0;

    StilWindow* win = &g_wins[slot];
    memset(win, 0, sizeof(StilWindow));
    win->bg = 0x101418;
    win->open = 1;
    win->scale = 1.0;

    const char* t = (title && title[0]) ? title : "Still";
    RECT r = {0, 0, (LONG)w, (LONG)h};
    AdjustWindowRect(&r, WS_OVERLAPPEDWINDOW, FALSE);
    int ww = (int)(r.right - r.left);
    int wh = (int)(r.bottom - r.top);

    /* wide-string creation so UTF-8 titles (em dashes, accents, CJK)
       render correctly in the title bar */
    wchar_t* wt = still_wide(t);
    HWND hwnd = CreateWindowExW(0, L"StillWindow", wt ? wt : L"Still",
                                WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, ww, wh,
                                NULL, NULL, GetModuleHandleA(NULL), NULL);
    free(wt);
    if (!hwnd) {
        win->open = 0;
        return 0;
    }
    win->hwnd = hwnd;
    win->w = (int)w;
    win->h = (int)h;
    win->dpi = (int)gui_window_dpi(hwnd);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    UpdateWindow(hwnd);
    return (int64_t)(uintptr_t)hwnd;
}

/* Set the logical->physical scale for a window. Programs draw in logical
   96-DPI pixels (like a web page's CSS pixels); the runtime multiplies
   every coordinate, size, radius and font size by this factor and converts
   mouse/wheel input back to logical units. Set it each frame from
   dpi/96 to follow monitor changes. */
void still_gui_set_scale(int64_t handle, double scale) {
    StilWindow* w = gui_win(handle);
    if (!w) return;
    if (scale < 0.25) scale = 0.25;
    if (scale > 8.0) scale = 8.0;
    w->scale = scale;
}

int64_t still_gui_window_open(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return (w && w->open) ? 1 : 0;
}

void still_gui_close_window(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return;
    if (w->bmp) DeleteObject(w->bmp);
    if (w->mem) DeleteDC(w->mem);
    for (int i = 0; i < w->font_count; i++) DeleteObject(w->fonts[i].font);
    if (w->hwnd) DestroyWindow(w->hwnd);
    memset(w, 0, sizeof(StilWindow));
}

void still_gui_set_title(int64_t handle, const char* title) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->hwnd) return;
    wchar_t* wt = still_wide(title ? title : "");
    if (wt) {
        SetWindowTextW(w->hwnd, wt);
        free(wt);
    }
}

void still_gui_set_bg(int64_t handle, int64_t rgb) {
    StilWindow* w = gui_win(handle);
    if (w) w->bg = (int)(rgb & 0xFFFFFF);
}

static COLORREF gui_rgb(int64_t rgb) {
    /* Still colors are 0xRRGGBB; GDI wants 0x00BBGGRR */
    long r = (long)((rgb >> 16) & 0xFF);
    long g = (long)((rgb >> 8) & 0xFF);
    long b = (long)(rgb & 0xFF);
    return RGB(r, g, b);
}

static int gui_ensure_backbuffer(StilWindow* w) {
    RECT cr;
    GetClientRect(w->hwnd, &cr);
    int cw = cr.right - cr.left;
    int ch = cr.bottom - cr.top;
    if (cw < 1) cw = 1;
    if (ch < 1) ch = 1;
    if (w->mem && cw == w->w && ch == w->h) return 1;
    HDC wdc = GetDC(w->hwnd);
    HDC mem = CreateCompatibleDC(wdc);
    HBITMAP bmp = CreateCompatibleBitmap(wdc, cw, ch);
    ReleaseDC(w->hwnd, wdc);
    if (!mem || !bmp) {
        if (mem) DeleteDC(mem);
        if (bmp) DeleteObject(bmp);
        return 0;
    }
    SelectObject(mem, bmp);
    if (w->mem) {
        DeleteObject(w->bmp);
        DeleteDC(w->mem);
    }
    w->mem = mem;
    w->bmp = bmp;
    w->w = cw;
    w->h = ch;
    SetBkMode(w->mem, TRANSPARENT);
    return 1;
}

static void gui_pump(StilWindow* w) {
    MSG msg;
    while (PeekMessageA(&msg, w->hwnd, 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
        if (!w->open) break;
    }
    if (!w->open) return;
    /* poll mouse position and button */
    POINT p;
    GetCursorPos(&p);
    ScreenToClient(w->hwnd, &p);
    w->mouse_x = p.x;
    w->mouse_y = p.y;
    RECT cr;
    GetClientRect(w->hwnd, &cr);
    int inside = (p.x >= cr.left && p.x < cr.right && p.y >= cr.top && p.y < cr.bottom);
    int down = (GetAsyncKeyState(VK_LBUTTON) & 0x8000) ? 1 : 0;
    w->click_edge = (down && !w->ldown && inside) ? 1 : 0;
    w->ldown = down;
    /* keyboard edges from window messages */
    for (int i = 0; i < 256; i++) {
        w->keys_edge[i] = (w->keys[i] && !w->keys_prev[i]) ? 1 : 0;
        w->keys_prev[i] = w->keys[i];
    }
    if (!IsWindow(w->hwnd)) w->open = 0;
}

int64_t still_gui_begin_frame(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->open) return 0;
    gui_pump(w);
    if (!w->open) return 0;
    if (!gui_ensure_backbuffer(w)) return 0;
    RECT r = {0, 0, w->w, w->h};
    HBRUSH b = CreateSolidBrush(gui_rgb(w->bg));
    FillRect(w->mem, &r, b);
    DeleteObject(b);
    return 1;
}

void still_gui_end_frame(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !w->open) return;
    HDC wdc = GetDC(w->hwnd);
    if (wdc) {
        BitBlt(wdc, 0, 0, w->w, w->h, w->mem, 0, 0, SRCCOPY);
        ReleaseDC(w->hwnd, wdc);
    }
    /* clear per-frame input accumulators */
    w->input_len = 0;
    w->input[0] = 0;
    w->wheel = 0;
    w->click_edge = 0;
}

int64_t still_gui_width(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return 0;
    return (int64_t)((double)w->w / w->scale);
}

int64_t still_gui_height(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return 0;
    return (int64_t)((double)w->h / w->scale);
}

int64_t still_gui_mouse_x(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return -1;
    return (int64_t)((double)w->mouse_x / w->scale);
}

int64_t still_gui_dpi(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (w && w->dpi > 0) return (int64_t)w->dpi;
    return still_gui_system_dpi();
}

int64_t still_gui_system_dpi(void) {
    typedef UINT (WINAPI *GetDpiSysFn)(void);
    GetDpiSysFn f = (GetDpiSysFn)GetProcAddress(GetModuleHandleA("user32.dll"), "GetDpiForSystem");
    if (f) {
        UINT d = f();
        if (d > 0) return (int64_t)d;
    }
    HDC hdc = GetDC(NULL);
    UINT d = (UINT)GetDeviceCaps(hdc, LOGPIXELSX);
    ReleaseDC(NULL, hdc);
    return d ? (int64_t)d : 96;
}

int64_t still_gui_mouse_y(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return -1;
    return (int64_t)((double)w->mouse_y / w->scale);
}

int64_t still_gui_mouse_down(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return (w && w->ldown) ? 1 : 0;
}

int64_t still_gui_mouse_clicked(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return (w && w->click_edge) ? 1 : 0;
}

int64_t still_gui_wheel_delta(int64_t handle) {
    StilWindow* w = gui_win(handle);
    if (!w) return 0;
    return (int64_t)((double)w->wheel / w->scale);
}

const char* still_gui_text_input(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return (w && w->input_len > 0) ? w->input : "";
}

int64_t still_gui_key(int64_t handle, int64_t vk) {
    StilWindow* w = gui_win(handle);
    if (!w || vk < 0 || vk > 255) return 0;
    return (GetAsyncKeyState((int)vk) & 0x8000) ? 1 : 0;
}

int64_t still_gui_key_pressed(int64_t handle, int64_t vk) {
    StilWindow* w = gui_win(handle);
    if (!w || vk < 0 || vk > 255) return 0;
    return w->keys_edge[vk] ? 1 : 0;
}

static HFONT gui_font_lookup(StilWindow* w, double size, int weight) {
    double slot = (weight > FW_NORMAL) ? -size : size;
    for (int i = 0; i < w->font_count; i++) {
        if (w->fonts[i].size == slot) return w->fonts[i].font;
    }
    if (size < 6) size = 6;
    if (size > 96) size = 96;
    HFONT f = CreateFontA(-(int)size, 0, 0, 0, weight, FALSE, FALSE, FALSE,
                          DEFAULT_CHARSET, OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS,
                          CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE, "Segoe UI");
    if (!f) return NULL;
    if (w->font_count < STIL_GUI_FONT_CACHE) {
        w->fonts[w->font_count].size = slot;
        w->fonts[w->font_count].font = f;
        w->font_count++;
        return f;
    }
    DeleteObject(f); /* cache full; use a transient font */
    return NULL;
}

void still_gui_text(int64_t handle, double x, double y, const char* text, int64_t rgb, double size) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !text || !text[0]) return;
    double sc = w->scale;
    HFONT f = gui_font_lookup(w, size * sc, FW_NORMAL);
    if (f) SelectObject(w->mem, f);
    SetTextColor(w->mem, gui_rgb(rgb));
    wchar_t* wt = still_wide(text);
    if (wt) {
        TextOutW(w->mem, (int)(x * sc), (int)(y * sc), wt, lstrlenW(wt));
        free(wt);
    }
}

void still_gui_text_bold(int64_t handle, double x, double y, const char* text, int64_t rgb, double size) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !text || !text[0]) return;
    double sc = w->scale;
    HFONT f = gui_font_lookup(w, size * sc, FW_SEMIBOLD);
    if (f) SelectObject(w->mem, f);
    SetTextColor(w->mem, gui_rgb(rgb));
    wchar_t* wt = still_wide(text);
    if (wt) {
        TextOutW(w->mem, (int)(x * sc), (int)(y * sc), wt, lstrlenW(wt));
        free(wt);
    }
}

int64_t still_gui_text_width(int64_t handle, const char* text, double size) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !text || !text[0]) return 0;
    double sc = w->scale;
    HFONT f = gui_font_lookup(w, size * sc, FW_NORMAL);
    if (f) SelectObject(w->mem, f);
    wchar_t* wt = still_wide(text);
    double width = 0.0;
    if (wt) {
        SIZE sz;
        if (GetTextExtentPoint32W(w->mem, wt, lstrlenW(wt), &sz)) width = (double)sz.cx / sc;
        free(wt);
    }
    return (int64_t)(width + 0.5);
}

void still_gui_fill_rect(int64_t handle, double x, double y, double w, double h, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    double sc = win->scale;
    RECT r = { (LONG)(x * sc), (LONG)(y * sc), (LONG)((x + w) * sc), (LONG)((y + h) * sc) };
    HBRUSH b = CreateSolidBrush(gui_rgb(rgb));
    FillRect(win->mem, &r, b);
    DeleteObject(b);
}

void still_gui_fill_round(int64_t handle, double x, double y, double w, double h, double rad, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    double sc = win->scale;
    x *= sc; y *= sc; w *= sc; h *= sc; rad *= sc;
    if (rad < 1) rad = 1;
    if (rad * 2 > w) rad = w / 2;
    if (rad * 2 > h) rad = h / 2;
    HBRUSH b = CreateSolidBrush(gui_rgb(rgb));
    HPEN pen = CreatePen(PS_NULL, 1, 0);
    HPEN old_pen = (HPEN)SelectObject(win->mem, pen);
    HBRUSH old_brush = (HBRUSH)SelectObject(win->mem, b);
    RoundRect(win->mem, (int)x, (int)y, (int)(x + w), (int)(y + h), (int)(rad * 2), (int)(rad * 2));
    SelectObject(win->mem, old_brush);
    SelectObject(win->mem, old_pen);
    DeleteObject(b);
    DeleteObject(pen);
}

void still_gui_fill_circle(int64_t handle, double cx, double cy, double rad, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    double sc = win->scale;
    cx *= sc; cy *= sc; rad *= sc;
    HBRUSH b = CreateSolidBrush(gui_rgb(rgb));
    HPEN pen = CreatePen(PS_NULL, 1, 0);
    HPEN old_pen = (HPEN)SelectObject(win->mem, pen);
    HBRUSH old_brush = (HBRUSH)SelectObject(win->mem, b);
    Ellipse(win->mem, (int)(cx - rad), (int)(cy - rad), (int)(cx + rad), (int)(cy + rad));
    SelectObject(win->mem, old_brush);
    SelectObject(win->mem, old_pen);
    DeleteObject(b);
    DeleteObject(pen);
}

void still_gui_stroke_circle(int64_t handle, double cx, double cy, double rad, double width, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    double sc = win->scale;
    cx *= sc; cy *= sc; rad *= sc; width *= sc;
    HPEN pen = CreatePen(PS_SOLID, (int)(width < 1 ? 1 : width), gui_rgb(rgb));
    HPEN old_pen = (HPEN)SelectObject(win->mem, pen);
    HBRUSH old_brush = (HBRUSH)SelectObject(win->mem, GetStockObject(NULL_BRUSH));
    Ellipse(win->mem, (int)(cx - rad), (int)(cy - rad), (int)(cx + rad), (int)(cy + rad));
    SelectObject(win->mem, old_brush);
    SelectObject(win->mem, old_pen);
    DeleteObject(pen);
}

void still_gui_clip_push(int64_t handle, double x, double y, double w, double h) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    double sc = win->scale;
    SaveDC(win->mem);
    IntersectClipRect(win->mem, (int)(x * sc), (int)(y * sc), (int)((x + w) * sc), (int)((y + h) * sc));
}

void still_gui_clip_pop(int64_t handle) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    RestoreDC(win->mem, -1);
}

/* ============================================================ audio: WAV playback + melody synth */

static HMODULE g_audio_winmm = NULL;
typedef BOOL (WINAPI *PlaySoundFn)(LPCSTR, HMODULE, DWORD);
static PlaySoundFn g_play_sound = NULL;
static unsigned char* g_audio_buf = NULL;
#define STIL_SND_ASYNC 0x0001
#define STIL_SND_NODEFAULT 0x0002
#define STIL_SND_MEMORY 0x0004
#define STIL_SND_LOOP 0x0008
#define STIL_SND_FILENAME 0x00020000

int64_t still_audio_available(void) {
    if (g_play_sound) return 1;
    g_audio_winmm = LoadLibraryA("winmm.dll");
    if (!g_audio_winmm) return 0;
    g_play_sound = (PlaySoundFn)GetProcAddress(g_audio_winmm, "PlaySoundA");
    return g_play_sound ? 1 : 0;
}

void still_audio_stop(void) {
    if (g_play_sound) g_play_sound(NULL, NULL, 0);
}

int64_t still_audio_play_file(const char* path, int64_t loop) {
    if (!still_audio_available()) return -1;
    DWORD flags = STIL_SND_FILENAME | STIL_SND_ASYNC | STIL_SND_NODEFAULT | (loop ? STIL_SND_LOOP : 0);
    return g_play_sound(path, NULL, flags) ? 0 : -1;
}

/* Synthesize a short melody into a 16-bit mono 22050 Hz WAV held in memory.
   spec format: "freq:ms,freq:ms,..." (Hz:milliseconds pairs, comma separated;
   a frequency of 0 is a rest). Each note gets an 8 ms fade at both ends to
   avoid clicks. */
int64_t still_audio_play_melody(const char* spec, int64_t loop) {
    if (!still_audio_available()) return -1;
    if (!spec || !spec[0]) return -1;
    const int SR = 22050;
    long total_ms = 0;
    const char* p = spec;
    while (*p) {
        char* endp = NULL;
        double f = strtod(p, &endp);
        (void)f;
        if (endp == p) break;
        while (*endp == ' ') endp++;
        if (*endp != ':') break;
        long ms = strtol(endp + 1, &endp, 10);
        if (ms < 20) ms = 20;
        if (ms > 10000) ms = 10000;
        total_ms += ms;
        while (*endp == ' ') endp++;
        if (*endp == ',') { p = endp + 1; continue; }
        break;
    }
    if (total_ms <= 0) return -1;
    size_t n_samples = (size_t)((long long)SR * total_ms / 1000);
    size_t data_size = n_samples * 2;
    size_t file_size = 44 + data_size;
    unsigned char* buf = (unsigned char*)malloc(file_size);
    if (!buf) return -1;
    unsigned char* q = buf;
    /* 44-byte canonical WAV header, 16-bit mono PCM */
    memcpy(q, "RIFF", 4); q += 4;
    unsigned long sz32 = (unsigned long)(file_size - 8);
    memcpy(q, &sz32, 4); q += 4;
    memcpy(q, "WAVE", 4); q += 4;
    memcpy(q, "fmt ", 4); q += 4;
    unsigned long fmt_sz = 16;
    memcpy(q, &fmt_sz, 4); q += 4;
    unsigned short fmt = 1, ch = 1, bits = 16;
    unsigned long rate = (unsigned long)SR;
    memcpy(q, &fmt, 2); q += 2;
    memcpy(q, &ch, 2); q += 2;
    memcpy(q, &rate, 4); q += 4;
    unsigned long byte_rate = rate * 2;
    memcpy(q, &byte_rate, 4); q += 4;
    unsigned short align = 2;
    memcpy(q, &align, 2); q += 2;
    memcpy(q, &bits, 2); q += 2;
    memcpy(q, "data", 4); q += 4;
    unsigned long dsz = (unsigned long)data_size;
    memcpy(q, &dsz, 4); q += 4;
    /* samples */
    double amp = 11000.0;
    double fade = SR * 0.008;
    p = spec;
    size_t pos = 0;
    while (*p && pos < n_samples) {
        char* endp = NULL;
        double freq = strtod(p, &endp);
        if (endp == p) break;
        if (freq < 0) freq = 0;
        if (freq > 20000) freq = 20000;
        while (*endp == ' ') endp++;
        if (*endp != ':') break;
        long ms = strtol(endp + 1, &endp, 10);
        if (ms < 20) ms = 20;
        if (ms > 10000) ms = 10000;
        size_t ns = (size_t)((long long)SR * ms / 1000);
        if (pos + ns > n_samples) ns = n_samples - pos;
        for (size_t i = 0; i < ns; i++) {
            double t = (double)i / (double)SR;
            double v = (freq > 0) ? sin(2.0 * 3.14159265358979 * freq * t) : 0.0;
            double env = 1.0;
            if ((double)i < fade) env = (double)i / fade;
            if ((double)(ns - i) < fade) env = (double)(ns - i) / fade;
            short s = (short)(v * env * amp);
            memcpy(q + (pos + i) * 2, &s, 2);
        }
        pos += ns;
        while (*endp == ' ') endp++;
        if (*endp == ',') { p = endp + 1; continue; }
        break;
    }
    free(g_audio_buf);
    g_audio_buf = buf; /* keep alive while PlaySound reads it */
    DWORD flags = STIL_SND_MEMORY | STIL_SND_ASYNC | STIL_SND_NODEFAULT | (loop ? STIL_SND_LOOP : 0);
    return g_play_sound((LPCSTR)buf, NULL, flags) ? 0 : -1;
}

/* ============================================================ media: image loading via GDI+ */

static HMODULE g_media_gdiplus = NULL;
static int g_media_ok = 0;

typedef long GpStatus;
typedef void GpImage;
typedef void GpGraphics;

static GpStatus (*p_GdipLoadImageFromFile)(const wchar_t*, GpImage**) = NULL;
static GpStatus (*p_GdipDisposeImage)(GpImage*) = NULL;
static GpStatus (*p_GdipGetImageWidth)(GpImage*, unsigned int*) = NULL;
static GpStatus (*p_GdipGetImageHeight)(GpImage*, unsigned int*) = NULL;
static GpStatus (*p_GdipCreateFromHDC)(HDC, GpGraphics**) = NULL;
static GpStatus (*p_GdipDeleteGraphics)(GpGraphics*) = NULL;
static GpStatus (*p_GdipDrawImageRectI)(GpGraphics*, GpImage*, int, int, int, int) = NULL;
static GpStatus (*p_GdipSetInterpolationMode)(GpGraphics*, int) = NULL;
static GpStatus (*p_GdipSetSmoothingMode)(GpGraphics*, int) = NULL;

int64_t still_media_available(void) {
    if (g_media_ok) return 1;
    g_media_gdiplus = LoadLibraryA("gdiplus.dll");
    if (!g_media_gdiplus) return 0;
    typedef long (*StartupFn)(unsigned long long*, const void*, void*);
    StartupFn startup = (StartupFn)GetProcAddress(g_media_gdiplus, "GdiplusStartup");
    if (!startup) return 0;
    /* GdiplusStartupInput: version, debug callback, suppress-background, suppress-codecs */
    struct { unsigned long ver; void* dbg; int sup_bg; int sup_ext; } in;
    in.ver = 1; in.dbg = NULL; in.sup_bg = 0; in.sup_ext = 0;
    unsigned long long token = 0;
    if (startup(&token, &in, NULL) != 0) return 0;
    p_GdipLoadImageFromFile = (void*)GetProcAddress(g_media_gdiplus, "GdipLoadImageFromFile");
    p_GdipDisposeImage = (void*)GetProcAddress(g_media_gdiplus, "GdipDisposeImage");
    p_GdipGetImageWidth = (void*)GetProcAddress(g_media_gdiplus, "GdipGetImageWidth");
    p_GdipGetImageHeight = (void*)GetProcAddress(g_media_gdiplus, "GdipGetImageHeight");
    p_GdipCreateFromHDC = (void*)GetProcAddress(g_media_gdiplus, "GdipCreateFromHDC");
    p_GdipDeleteGraphics = (void*)GetProcAddress(g_media_gdiplus, "GdipDeleteGraphics");
    p_GdipDrawImageRectI = (void*)GetProcAddress(g_media_gdiplus, "GdipDrawImageRectI");
    p_GdipSetInterpolationMode = (void*)GetProcAddress(g_media_gdiplus, "GdipSetInterpolationMode");
    p_GdipSetSmoothingMode = (void*)GetProcAddress(g_media_gdiplus, "GdipSetSmoothingMode");
    if (!p_GdipLoadImageFromFile || !p_GdipDisposeImage || !p_GdipGetImageWidth || !p_GdipGetImageHeight ||
        !p_GdipCreateFromHDC || !p_GdipDeleteGraphics || !p_GdipDrawImageRectI) return 0;
    g_media_ok = 1;
    return 1;
}

int64_t still_media_load(const char* path) {
    if (!still_media_available()) return 0;
    wchar_t* wp = still_wide(path);
    if (!wp) return 0;
    GpImage* img = NULL;
    GpStatus st = p_GdipLoadImageFromFile(wp, &img);
    free(wp);
    if (st != 0 || !img) return 0;
    return (int64_t)(uintptr_t)img;
}

void still_media_free(int64_t img) {
    if (img && p_GdipDisposeImage) p_GdipDisposeImage((GpImage*)(uintptr_t)img);
}

int64_t still_media_width(int64_t img) {
    unsigned int w = 0;
    if (img && p_GdipGetImageWidth) p_GdipGetImageWidth((GpImage*)(uintptr_t)img, &w);
    return (int64_t)w;
}

int64_t still_media_height(int64_t img) {
    unsigned int h = 0;
    if (img && p_GdipGetImageHeight) p_GdipGetImageHeight((GpImage*)(uintptr_t)img, &h);
    return (int64_t)h;
}

/* draw an image into a gui window's backbuffer (surfaced as gui.draw_image) */
void still_gui_draw_image(int64_t handle, int64_t img, double x, double y, double w, double h) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem || !img) return;
    if (!still_media_available()) return;
    double sc = win->scale;
    GpGraphics* g = NULL;
    if (p_GdipCreateFromHDC(win->mem, &g) != 0 || !g) return;
    if (p_GdipSetInterpolationMode) p_GdipSetInterpolationMode(g, 7 /* HighQualityBicubic */);
    if (p_GdipSetSmoothingMode) p_GdipSetSmoothingMode(g, 4 /* AntiAlias */);
    p_GdipDrawImageRectI(g, (GpImage*)(uintptr_t)img,
                         (int)(x * sc), (int)(y * sc), (int)(w * sc), (int)(h * sc));
    p_GdipDeleteGraphics(g);
}

/* ============================================================ video: MFPlay
   Real video playback (MP4/H.264) via Media Foundation's MFPlay, loaded
   dynamically from mf.dll. We create our own WS_CHILD window inside the
   app window, hand it to MFPCreateMediaPlayer, and move/resize it with
   SetWindowPos (the video fills that child window; MFPlay letterboxes to
   keep the aspect ratio). Audio is handled by the pipeline itself.
   Vtable slots counted from the Windows SDK mfplay.h declaration of
   IMFPMediaPlayer (Play=3 Pause=4 SetPosition=7 GetState=13 Shutdown=38). */

static void* g_mfp_player = NULL;   /* IMFPMediaPlayer* */
static HMODULE g_mf_dll = NULL;
static HWND g_video_child = NULL;
static int g_mf_inited = 0;

#define MFP_PLAY_SLOT 3
#define MFP_PAUSE_SLOT 4
#define MFP_SETPOSITION_SLOT 7
#define MFP_GETSTATE_SLOT 13
#define MFP_SHUTDOWN_SLOT 38

/* MFP_POSITIONTYPE_100NS = {00000000-0000-0000-0000-000000000000} */
static const unsigned char g_mfp_pos_guid[16] = {0};

typedef long (__stdcall *MfpFn0)(void*);

static void* mfp_slot(int idx) {
    if (!g_mfp_player) return NULL;
    void** vt = *(void***)g_mfp_player;
    return vt ? vt[idx] : NULL;
}

int64_t still_media_video_available(void) {
    if (g_mf_dll) return 1;
    g_mf_dll = LoadLibraryA("mf.dll");
    return g_mf_dll ? 1 : 0;
}

int64_t still_media_video_play(const char* path, int64_t hwnd) {
    still_media_video_stop();
    if (!path || !path[0] || !hwnd) return 0;
    if (!still_media_video_available()) return 0;
    if (!g_mf_inited) {
        typedef long (__stdcall *MfStartupFn)(unsigned long, unsigned long);
        HMODULE mfplat = LoadLibraryA("mfplat.dll");
        if (mfplat) {
            MfStartupFn startup = (MfStartupFn)GetProcAddress(mfplat, "MFStartup");
            if (startup) startup(0x20070 /*MF_VERSION*/, 0);
        }
        g_mf_inited = 1;
    }
    typedef long (__stdcall *CreatePlayerFn)(const unsigned short*, int, void*, void*, void**, void**);
    CreatePlayerFn create = (CreatePlayerFn)GetProcAddress((HMODULE)g_mf_dll, "MFPCreateMediaPlayer");
    if (!create) return 0;
    /* dedicated child window for the video, positioned by video_rect();
       created hidden and shown once playback actually starts */
    HWND child = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                                 0, 0, 16, 16, (HWND)(uintptr_t)hwnd, NULL,
                                 GetModuleHandleA(NULL), NULL);
    if (!child) return 0;

    unsigned short* wpath = still_wide(path);
    if (!wpath) { DestroyWindow(child); return 0; }
    void* player = NULL;
    long hr = create(wpath, 1 /*start playback immediately*/, 0,
                     NULL /*no callback needed: state is polled via GetState*/,
                     child, &player);
    free(wpath);
    if (hr != 0 || !player) {
        DestroyWindow(child);
        return 0;
    }
    ShowWindow(child, SW_SHOW);
    g_video_child = child;
    g_mfp_player = player;
    return 1;
}

/* Position the video child window. Coordinates are LOGICAL gui pixels of
   the given window; scaled to physical with the window's gui scale. */
void still_media_video_rect(int64_t gui_handle, double x, double y, double w, double h) {
    if (!g_video_child) return;
    double sc = 1.0;
    StilWindow* gw = gui_win(gui_handle);
    if (gw) sc = gw->scale;
    SetWindowPos(g_video_child, NULL, (int)(x * sc), (int)(y * sc),
                 (int)(w * sc + 0.5), (int)(h * sc + 0.5),
                 SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS);
}

int64_t still_media_video_state(void) {
    if (!g_mfp_player) return 0;
    typedef long (__stdcall *MfpFnInt)(void*, int*);
    MfpFnInt state_fn = (MfpFnInt)mfp_slot(MFP_GETSTATE_SLOT);
    if (!state_fn) return 0;
    int st = 0;
    if (state_fn(g_mfp_player, &st) != 0) return 0;
    /* MFP_MEDIAPLAYER_STATE: 0 empty 1 stopped 2 playing 3 paused 4 shutdown */
    if (st == 2) return 1;
    if (st == 3) return 2;
    if (st == 1) return 3;
    return 0;
}

void still_media_video_rewind(void) {
    if (!g_mfp_player) return;
    MfpFn0 play = (MfpFn0)mfp_slot(MFP_PLAY_SLOT);
    typedef long (__stdcall *MfpFnPos)(void*, const void*, void*);
    MfpFnPos setpos = (MfpFnPos)mfp_slot(MFP_SETPOSITION_SLOT);
    if (setpos) {
        /* PROPVARIANT for VT_I8 0ns: vt=20, quadpart at offset 8 */
        unsigned char pv[32];
        memset(pv, 0, sizeof(pv));
        pv[0] = 20;
        setpos(g_mfp_player, (const void*)g_mfp_pos_guid, (void*)pv);
    }
    if (play) play(g_mfp_player);
}

void still_media_video_pause(void) {
    if (!g_mfp_player) return;
    MfpFn0 pause = (MfpFn0)mfp_slot(MFP_PAUSE_SLOT);
    if (pause) pause(g_mfp_player);
}

void still_media_video_resume(void) {
    if (!g_mfp_player) return;
    MfpFn0 play = (MfpFn0)mfp_slot(MFP_PLAY_SLOT);
    if (play) play(g_mfp_player);
}

void still_media_video_stop(void) {
    if (g_mfp_player) {
        MfpFn0 shutdown_fn = (MfpFn0)mfp_slot(MFP_SHUTDOWN_SLOT);
        if (shutdown_fn) shutdown_fn(g_mfp_player);
        MfpFn0 release_fn = (MfpFn0)mfp_slot(2);
        if (release_fn) release_fn(g_mfp_player);
        g_mfp_player = NULL;
    }
    if (g_video_child) {
        DestroyWindow(g_video_child);
        g_video_child = NULL;
    }
}

/* ============================================================ webview: WebView2 login dialog
   An embedded Edge (WebView2) window for in-app OAuth-style logins. The app
   calls still_webview_login(start_url, poll_url): a real browser window
   opens, the user signs in normally (2FA, checkpoints, "remember me" all
   work), and the runtime polls the WebView2 cookie manager until the wanted
   cookies appear. Values are read back with still_webview_cookie(name).
   Everything is loaded dynamically: WebView2Loader.dll (shipped next to the
   exe) and ole32 for COM init. No SDK headers needed: WebView2's COM
   vtables are invoked by slot index, verified against WebView2.h from the
   Microsoft.Web.WebView2 1.0.2903.40 package. */

#define WV2_COOKIE_MAX 16

typedef struct {
    char name[64];
    char value[512];
} StilWVCookie;

static void* g_wv2_loader = NULL;
static char g_wv2_error[512] = {0};
static char g_wv2_version[128] = {0};
static StilWVCookie g_wv2_cookies[WV2_COOKIE_MAX];
static int g_wv2_cookie_count = 0;

/* live dialog state */
static void* g_wv2_env_obj = NULL;
static void* g_wv2_ctrl_obj = NULL;
static void* g_wv2_web_obj = NULL;
static void* g_wv2_web2_obj = NULL;   /* ICoreWebView2_2 via QI (cookie manager lives here) */
static void* g_wv2_pending_cm = NULL;
static HWND g_wv2_hwnd = NULL;
static int g_wv2_status = 0;   /* 0 running, 1 cookies captured, 2 user closed, 3 error */
static unsigned short* g_wv2_poll_url_w = NULL;
static unsigned short* g_wv2_start_url_w = NULL;
static char g_wv2_wanted[64] = "sessionid";
static DWORD g_wv2_deadline = 0;      /* GetTickCount deadline, 0 = wait forever */
static int g_wv2_poll_count = 0;      /* completed GetCookies round-trips (CI probe) */
static int g_wv2_teardown_started = 0;

/* IID_ICoreWebView2_2 = {9E8F0CF8-E670-4B5E-B2BC-73E061E3184C} */
static const unsigned char g_wv2_iid_web2[16] = {
    0xF8, 0x0C, 0x8F, 0x9E, 0x70, 0xE6, 0x5E, 0x4B,
    0xB2, 0xBC, 0x73, 0xE0, 0x61, 0xE3, 0x18, 0x4C
};

/* vtable slot indices (counted from WebView2.h interface declarations) */
#define WV2_WEBVIEW_NAVIGATE 5
#define WV2_WEBVIEW2_GET_COOKIEMANAGER 66   /* ICoreWebView2_2: IUnknown(3) + ICoreWebView2(58) + 5th member */
#define WV2_CTRL_PUT_ISVISIBLE 4
#define WV2_CTRL_PUT_BOUNDS 6
#define WV2_CTRL_CLOSE 24
#define WV2_CTRL_GET_WEBVIEW 25
#define WV2_ENV_CREATE_CONTROLLER 3
#define WV2_CM_GET_COOKIES 5
#define WV2_CLIST_COUNT 3
#define WV2_CLIST_VALUE_AT 4
#define WV2_COOKIE_GET_NAME 3
#define WV2_COOKIE_GET_VALUE 4

typedef long (__stdcall *WV2Fn0)(void*);
typedef long (__stdcall *WV2FnV)(void*, void**);
typedef long (__stdcall *WV2FnVP)(void*, void*, void*);
typedef long (__stdcall *WV2FnW)(void*, const unsigned short*);
typedef long (__stdcall *WV2FnHwnd)(void*, HWND, void*);
typedef long (__stdcall *WV2FnInt)(void*, int);
typedef long (__stdcall *WV2FnRect)(void*, RECT);
typedef long (__stdcall *WV2FnU32P)(void*, unsigned int*);
typedef long (__stdcall *WV2FnU32VP)(void*, unsigned int, void**);

#ifndef SPI_GETWORKAREA
#define SPI_GETWORKAREA 0x0030
#endif
#ifndef WM_TIMER
#define WM_TIMER 0x0113
#endif

static void* wv2_slot(void* obj, int idx) {
    if (!obj) return NULL;
    void** vt = *(void***)obj;
    if (!vt) return NULL;
    return vt[idx];
}

/* env-gated stage tracing (STILL_WV2_TRACE=1) for CI diagnosis, same
   pattern as the net layer's STILL_NET_TRACE */
static int g_wv2_trace = -1;
static void wv2_trace(const char* stage) {
    if (g_wv2_trace < 0) g_wv2_trace = (getenv("STILL_WV2_TRACE") != NULL) ? 1 : 0;
    if (g_wv2_trace) fprintf(stderr, "still_webview: %s\n", stage);
}

static unsigned long wv2_release(void* obj) {
    WV2Fn0 f = (WV2Fn0)wv2_slot(obj, 2);
    return f ? f(obj) : 0;
}

/* CoTaskMemFree — strings returned by WebView2 property getters are
   allocated with the COM task allocator. ole32 stays loaded forever
   (harmless) so later frees still work. */
static void wv2_cofree(void* p) {
    if (!p) return;
    static void (__stdcall *cofree_fn)(void*) = NULL;
    if (!cofree_fn) {
        HMODULE ole = GetModuleHandleA("ole32.dll");
        if (!ole) ole = LoadLibraryA("ole32.dll");
        if (ole) cofree_fn = (void (__stdcall *)(void*))GetProcAddress(ole, "CoTaskMemFree");
    }
    if (cofree_fn) cofree_fn(p);
}

static char* wv2_get_cookie_value(const char* name) {
    if (!name) return NULL;
    for (int i = 0; i < g_wv2_cookie_count; i++) {
        if (strcmp(g_wv2_cookies[i].name, name) == 0) {
            size_t n = strlen(g_wv2_cookies[i].value) + 1;
            char* out = (char*)malloc(n);
            if (out) memcpy(out, g_wv2_cookies[i].value, n);
            return out;
        }
    }
    return NULL;
}

/* COM handler objects: the loader/WebView2 call Invoke directly on the
   pointer we pass, so the object is just a vtable pointer. */
typedef struct { void* vtbl; } StilHandler;

static long __stdcall wv2_qi(void* self, const void* riid, void** out) {
    (void)self; (void)riid;
    if (out) *out = NULL;
    return (long)0x80004002L; /* E_NOINTERFACE (handlers are never QI'd) */
}
static unsigned long __stdcall wv2_ref(void* self) { (void)self; return 1; }

static void* g_wv2_env_vt[4];
static void* g_wv2_ctrl_vt[4];
static void* g_wv2_cookies_vt[4];
static int g_wv2_vts_ready = 0;

static long __stdcall wv2_env_invoke(void* self, long result, void* env);
static long __stdcall wv2_ctrl_invoke(void* self, long result, void* ctrl);
static long __stdcall wv2_cookies_invoke(void* self, long result, void* list);

static void wv2_init_vtables(void) {
    if (g_wv2_vts_ready) return;
    g_wv2_env_vt[0] = (void*)wv2_qi;
    g_wv2_env_vt[1] = (void*)wv2_ref;
    g_wv2_env_vt[2] = (void*)wv2_ref;
    g_wv2_env_vt[3] = (void*)wv2_env_invoke;
    g_wv2_ctrl_vt[0] = (void*)wv2_qi;
    g_wv2_ctrl_vt[1] = (void*)wv2_ref;
    g_wv2_ctrl_vt[2] = (void*)wv2_ref;
    g_wv2_ctrl_vt[3] = (void*)wv2_ctrl_invoke;
    g_wv2_cookies_vt[0] = (void*)wv2_qi;
    g_wv2_cookies_vt[1] = (void*)wv2_ref;
    g_wv2_cookies_vt[2] = (void*)wv2_ref;
    g_wv2_cookies_vt[3] = (void*)wv2_cookies_invoke;
    g_wv2_vts_ready = 1;
}

static StilHandler g_wv2_env_handler = { NULL };
static StilHandler g_wv2_ctrl_handler = { NULL };
static StilHandler g_wv2_cookies_handler = { NULL };

static void wv2_set_cookie(const char* name, const char* value) {
    if (!name || !value) return;
    for (int i = 0; i < g_wv2_cookie_count; i++) {
        if (strcmp(g_wv2_cookies[i].name, name) == 0) {
            snprintf(g_wv2_cookies[i].value, sizeof(g_wv2_cookies[i].value), "%s", value);
            return;
        }
    }
    if (g_wv2_cookie_count >= WV2_COOKIE_MAX) return;
    StilWVCookie* c = &g_wv2_cookies[g_wv2_cookie_count];
    snprintf(c->name, sizeof(c->name), "%s", name);
    snprintf(c->value, sizeof(c->value), "%s", value);
    g_wv2_cookie_count++;
}

static void wv2_resize(void) {
    if (!g_wv2_ctrl_obj || !g_wv2_hwnd) return;
    RECT cr;
    if (!GetClientRect(g_wv2_hwnd, &cr)) return;
    WV2FnRect bounds_fn = (WV2FnRect)wv2_slot(g_wv2_ctrl_obj, WV2_CTRL_PUT_BOUNDS);
    if (bounds_fn) bounds_fn(g_wv2_ctrl_obj, cr);
}

static void wv2_poll_cookies(void) {
    if (g_wv2_status != 0 || !g_wv2_web2_obj) return;
    /* auto-close when the optional deadline passes (CI probes): treat it
       as "window closed without the cookie appearing". Wrap-safe unsigned
       compare: (now - deadline) small means the deadline has passed. */
    if (g_wv2_deadline) {
        DWORD now = GetTickCount();
        if ((DWORD)(now - g_wv2_deadline) < 0x80000000UL) {
            wv2_trace("deadline reached, closing");
            snprintf(g_wv2_error, sizeof(g_wv2_error), "timed out waiting for the cookie");
            g_wv2_status = 2;
            if (g_wv2_hwnd) PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
            return;
        }
    }
    /* keep the cookie manager alive until its async result arrives:
       release the previous poll's manager only now */
    if (g_wv2_pending_cm) {
        wv2_release(g_wv2_pending_cm);
        g_wv2_pending_cm = NULL;
    }
    /* get_CookieManager is a member of ICoreWebView2_2; we hold a
       QI-obtained web2 pointer, so the slot is guaranteed correct */
    WV2FnV get_cm = (WV2FnV)wv2_slot(g_wv2_web2_obj, WV2_WEBVIEW2_GET_COOKIEMANAGER);
    if (!get_cm) return;
    void* cm = NULL;
    if (get_cm(g_wv2_web2_obj, &cm) != 0 || !cm) return;
    WV2FnVP get_cookies = (WV2FnVP)wv2_slot(cm, WV2_CM_GET_COOKIES);
    if (get_cookies && g_wv2_poll_url_w) {
        get_cookies(cm, g_wv2_poll_url_w, (void*)&g_wv2_cookies_handler);
        g_wv2_pending_cm = cm;
    } else {
        wv2_release(cm);
    }
}

static long __stdcall wv2_cookies_invoke(void* self, long result, void* list) {
    (void)self;
    if (g_wv2_status != 0) { if (list) wv2_release(list); return 0; }
    if (result != 0 || !list) return 0;
    g_wv2_poll_count++;   /* the async cookie pipeline completed a round-trip */
    WV2FnU32P count_fn = (WV2FnU32P)wv2_slot(list, WV2_CLIST_COUNT);
    WV2FnU32VP at_fn = (WV2FnU32VP)wv2_slot(list, WV2_CLIST_VALUE_AT);
    if (count_fn && at_fn) {
        unsigned int count = 0;
        if (count_fn(list, &count) == 0) {
            if (count > 128) count = 128;
            for (unsigned int i = 0; i < count; i++) {
                void* cookie = NULL;
                if (at_fn(list, i, &cookie) != 0 || !cookie) continue;
                WV2FnV name_fn = (WV2FnV)wv2_slot(cookie, WV2_COOKIE_GET_NAME);
                WV2FnV value_fn = (WV2FnV)wv2_slot(cookie, WV2_COOKIE_GET_VALUE);
                if (name_fn && value_fn) {
                    void* wname_v = NULL;
                    void* wvalue_v = NULL;
                    if (name_fn(cookie, &wname_v) == 0 && wname_v &&
                        value_fn(cookie, &wvalue_v) == 0 && wvalue_v) {
                        unsigned short* wname = (unsigned short*)wname_v;
                        unsigned short* wvalue = (unsigned short*)wvalue_v;
                        char* name = still_narrow(wname);
                        char* value = still_narrow(wvalue);
                        if (name && value) wv2_set_cookie(name, value);
                        free(name);
                        free(value);
                    }
                    if (wname_v) wv2_cofree(wname_v);
                    if (wvalue_v) wv2_cofree(wvalue_v);
                }
                wv2_release(cookie);
            }
        }
    }
    wv2_release(list);
    char* sid = wv2_get_cookie_value(g_wv2_wanted);
    if (sid && sid[0] != 0) {
        g_wv2_status = 1;
        PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
    }
    free(sid);
    return 0;
}

static long __stdcall wv2_env_invoke(void* self, long result, void* env) {
    (void)self;
    wv2_trace("environment completion");
    if (result != 0 || !env) {
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "webview environment failed (0x%08lx)", (unsigned long)result);
        g_wv2_status = 3;
        if (g_wv2_hwnd) PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
        return 0;
    }
    g_wv2_env_obj = env;
    WV2FnHwnd create = (WV2FnHwnd)wv2_slot(env, WV2_ENV_CREATE_CONTROLLER);
    if (!create) {
        snprintf(g_wv2_error, sizeof(g_wv2_error), "webview environment too old");
        g_wv2_status = 3;
        if (g_wv2_hwnd) PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
        return 0;
    }
    create(env, g_wv2_hwnd, (void*)&g_wv2_ctrl_handler);
    return 0;
}

static long __stdcall wv2_ctrl_invoke(void* self, long result, void* ctrl) {
    (void)self;
    wv2_trace("controller completion");
    if (result != 0 || !ctrl) {
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "webview controller failed (0x%08lx)", (unsigned long)result);
        g_wv2_status = 3;
        if (g_wv2_hwnd) PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
        return 0;
    }
    g_wv2_ctrl_obj = ctrl;
    WV2FnV get_web = (WV2FnV)wv2_slot(ctrl, WV2_CTRL_GET_WEBVIEW);
    WV2FnInt vis_fn = (WV2FnInt)wv2_slot(ctrl, WV2_CTRL_PUT_ISVISIBLE);
    if (get_web) get_web(ctrl, &g_wv2_web_obj);
    if (vis_fn) vis_fn(ctrl, 1);
    wv2_resize();
    /* QueryInterface the web object for ICoreWebView2_2 explicitly — the
       cookie manager call then goes through the right interface pointer
       instead of assuming the appended-vtable layout of the base pointer. */
    if (g_wv2_web_obj) {
        void* web2 = NULL;
        typedef long (__stdcall *QiFn)(void*, const void*, void**);
        QiFn qi2 = (QiFn)wv2_slot(g_wv2_web_obj, 0);
        wv2_trace("queryinterface ICoreWebView2_2");
        if (qi2 && qi2(g_wv2_web_obj, (const void*)g_wv2_iid_web2, &web2) == 0 && web2) {
            g_wv2_web2_obj = web2;
            wv2_trace("QI ok");
        } else {
            /* older runtime without _2: cookies unavailable */
            snprintf(g_wv2_error, sizeof(g_wv2_error),
                     "this WebView2 runtime is too old for cookie capture");
            g_wv2_status = 3;
            if (g_wv2_hwnd) PostMessageA(g_wv2_hwnd, WM_CLOSE, 0, 0);
            return 0;
        }
    }
    if (g_wv2_web_obj && g_wv2_start_url_w) {
        WV2FnW nav = (WV2FnW)wv2_slot(g_wv2_web_obj, WV2_WEBVIEW_NAVIGATE);
        if (nav) nav(g_wv2_web_obj, g_wv2_start_url_w);
    }
    if (g_wv2_hwnd) SetTimer(g_wv2_hwnd, 1, 1200, NULL);
    return 0;
}

/* Ordered teardown, idempotent. MUST run before the host window is
   destroyed: the login window is the parent of WebView2's browser child
   window, so DestroyWindow(parent) rips the browser away from the
   controller and the controller object dies — calling ctrl->Close() after
   that (as the old cleanup did) dereferenced a freed object and crashed
   with 0xC0000005 right after a successful login. Close() is async, so we
   pump the loop ~1.5s for the browser to finish dying while everything is
   still alive; only then may the caller destroy the window / release. */
static void wv2_begin_teardown(void) {
    if (g_wv2_teardown_started) return;
    g_wv2_teardown_started = 1;
    wv2_trace("teardown begin");
    if (g_wv2_hwnd && IsWindow(g_wv2_hwnd)) KillTimer(g_wv2_hwnd, 1);
    if (g_wv2_ctrl_obj) {
        WV2Fn0 close_fn = (WV2Fn0)wv2_slot(g_wv2_ctrl_obj, WV2_CTRL_CLOSE);
        if (close_fn) close_fn(g_wv2_ctrl_obj);
        wv2_trace("controller closed, pumping");
        for (int spin = 0; spin < 60; spin++) {
            MSG m;
            while (PeekMessageA(&m, NULL, 0, 0, PM_REMOVE)) {
                TranslateMessage(&m);
                DispatchMessageA(&m);
            }
            Sleep(25);
        }
        wv2_trace("pump done");
    }
}

static LRESULT CALLBACK wv2_login_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
        case WM_SIZE:
            wv2_resize();
            return 0;
        case WM_TIMER:
            if (wp == 1) wv2_poll_cookies();
            return 0;
        case WM_CLOSE:
            if (g_wv2_status == 0) g_wv2_status = 2;
            /* close the controller BEFORE destroying its parent window */
            wv2_begin_teardown();
            DestroyWindow(hwnd);
            return 0;
        case WM_DESTROY:
            KillTimer(hwnd, 1);
            PostQuitMessage(0);
            return 0;
    }
    return DefWindowProcA(hwnd, msg, wp, lp);
}

static void* wv2_load_proc(const char* name) {
    if (!g_wv2_loader) return NULL;
    return (void*)GetProcAddress((HMODULE)g_wv2_loader, name);
}

/* Load WebView2Loader.dll from the exe directory first, then the PATH. */
static void wv2_ensure_loader(void) {
    if (g_wv2_loader) return;
    char path[1024];
    DWORD n = GetModuleFileNameA(NULL, path, (DWORD)sizeof(path) - 1);
    if (n > 0 && n < sizeof(path)) {
        path[n] = 0;
        char* slash = strrchr(path, '\\');
        if (slash) {
            *(slash + 1) = 0;
            strncat(path, "WebView2Loader.dll", sizeof(path) - strlen(path) - 1);
            g_wv2_loader = (void*)LoadLibraryA(path);
        }
    }
    if (!g_wv2_loader) {
        g_wv2_loader = (void*)LoadLibraryA("WebView2Loader.dll");
    }
}

const char* still_webview_last_error(void) {
    return g_wv2_error[0] ? g_wv2_error : "";
}

const char* still_webview_cookie(const char* name) {
    if (!name) return "";
    for (int i = 0; i < g_wv2_cookie_count; i++) {
        if (strcmp(g_wv2_cookies[i].name, name) == 0) return g_wv2_cookies[i].value;
    }
    return "";
}

/* completed GetCookies round-trips of the last login() call — a non-zero
   value proves the env -> controller -> navigate -> QI -> cookie-manager
   -> async-callback chain executed (used by CI probes when the target
   site itself is unreachable from the runner) */
int64_t still_webview_poll_count(void) {
    return (int64_t)g_wv2_poll_count;
}

static int g_wv2_probe_done = 0;
static int g_wv2_probe_result = 0;

int64_t still_webview_available(void) {
    /* cached: the login screen calls this every frame and probing the
       registry each time would be wasteful */
    if (g_wv2_probe_done) return (int64_t)g_wv2_probe_result;
    g_wv2_error[0] = 0;
    wv2_ensure_loader();
    if (!g_wv2_loader) {
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "WebView2Loader.dll was not found next to wingram.exe");
        g_wv2_probe_done = 1;
        g_wv2_probe_result = 0;
        return 0;
    }
    typedef long (__stdcall *GetVerFn)(const unsigned short*, unsigned short**);
    GetVerFn get_ver = (GetVerFn)wv2_load_proc("GetAvailableCoreWebView2BrowserVersionString");
    if (!get_ver) {
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "WebView2Loader.dll is missing required exports");
        g_wv2_probe_done = 1;
        g_wv2_probe_result = 0;
        return 0;
    }
    unsigned short* ver = NULL;
    long hr = get_ver(NULL, &ver);
    if (hr != 0 || !ver || !ver[0]) {
        if (ver) wv2_cofree(ver);
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "the WebView2 runtime is not installed (Edge update missing)");
        g_wv2_probe_done = 1;
        g_wv2_probe_result = 0;
        return 0;
    }
    char* v = still_narrow(ver);
    if (v) {
        snprintf(g_wv2_version, sizeof(g_wv2_version), "%s", v);
        free(v);
    }
    wv2_cofree(ver);
    g_wv2_probe_done = 1;
    g_wv2_probe_result = 1;
    return 1;
}

const char* still_webview_runtime_version(void) {
    if (g_wv2_version[0] == 0) still_webview_available();
    return g_wv2_version;
}

/* Blocking: opens the login browser window and returns when the wanted
   cookie is captured, the window is closed, or (when timeout_ms > 0) the
   deadline passes.
   Returns 1 = cookie captured, 2 = window closed without login, 3 = error.
   `wanted` names the cookie that ends the dialog (e.g. "sessionid").
   poll_count() tells how many cookie round-trips completed (CI diagnostics). */
int64_t still_webview_login(const char* start_url, const char* poll_url, const char* wanted, int64_t timeout_ms) {
    g_wv2_error[0] = 0;
    g_wv2_cookie_count = 0;
    g_wv2_status = 0;
    g_wv2_env_obj = NULL;
    g_wv2_ctrl_obj = NULL;
    g_wv2_web_obj = NULL;
    g_wv2_web2_obj = NULL;
    g_wv2_poll_count = 0;
    g_wv2_deadline = 0;
    g_wv2_teardown_started = 0;
    if (timeout_ms > 0) g_wv2_deadline = GetTickCount() + (DWORD)timeout_ms;
    snprintf(g_wv2_wanted, sizeof(g_wv2_wanted), "%s",
             (wanted && wanted[0]) ? wanted : "sessionid");
    wv2_init_vtables();
    g_wv2_env_handler.vtbl = (void*)g_wv2_env_vt;
    g_wv2_ctrl_handler.vtbl = (void*)g_wv2_ctrl_vt;
    g_wv2_cookies_handler.vtbl = (void*)g_wv2_cookies_vt;

    if (still_webview_available() != 1) return 3;
    typedef long (__stdcall *CreateEnvFn)(const unsigned short*, const unsigned short*, void*, void*);
    CreateEnvFn create_env = (CreateEnvFn)wv2_load_proc("CreateCoreWebView2EnvironmentWithOptions");
    if (!create_env) {
        snprintf(g_wv2_error, sizeof(g_wv2_error), "WebView2Loader.dll missing CreateCoreWebView2EnvironmentWithOptions");
        return 3;
    }

    /* COM init (apartment-threaded, kept for the life of the process: the
       WebView2 host completes asynchronous work after this function returns,
       so uninitializing COM here would race its teardown) */
    HMODULE ole = LoadLibraryA("ole32.dll");
    typedef long (__stdcall *CoInitFn)(void*, unsigned long);
    CoInitFn co_init = ole ? (CoInitFn)GetProcAddress(ole, "CoInitializeEx") : NULL;
    if (co_init) co_init(NULL, 2 /*APARTMENTTHREADED*/);

    unsigned short* start_w = still_wide(start_url ? start_url : "");
    g_wv2_poll_url_w = still_wide(poll_url ? poll_url : "");
    g_wv2_start_url_w = start_w;
    if (!start_w || !g_wv2_poll_url_w) {
        free(start_w);
        g_wv2_start_url_w = NULL;
        snprintf(g_wv2_error, sizeof(g_wv2_error), "out of memory");
        return 3;
    }

    gui_register_class();
    gui_enable_dpi_awareness();

    WNDCLASSEXA wc;
    memset(&wc, 0, sizeof(wc));
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = wv2_login_proc;
    wc.hCursor = LoadCursor(NULL, (LPCSTR)IDC_ARROW);
    wc.hbrBackground = CreateSolidBrush(RGB(26, 26, 26));
    wc.lpszClassName = "StillWV2Login";
    wc.hInstance = GetModuleHandleA(NULL);
    RegisterClassExA(&wc);

    UINT sdpi = (UINT)still_gui_system_dpi();
    double scale = (double)sdpi / 96.0;
    int cw = (int)(480 * scale);
    int ch = (int)(640 * scale);
    RECT r;
    r.left = 0; r.top = 0; r.right = cw; r.bottom = ch;
    AdjustWindowRect(&r, WS_OVERLAPPEDWINDOW, FALSE);
    RECT wa;
    wa.left = 0; wa.top = 0; wa.right = 1280; wa.bottom = 800;
    SystemParametersInfoA(SPI_GETWORKAREA, 0, &wa, 0);
    int x = wa.left + (((wa.right - wa.left) - (r.right - r.left)) / 2);
    int y = wa.top + (((wa.bottom - wa.top) - (r.bottom - r.top)) / 2);

    HWND hwnd = CreateWindowExW(0, L"StillWV2Login", L"Wingram - Sign in to Instagram",
                                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                                x, y, r.right - r.left, r.bottom - r.top,
                                NULL, NULL, GetModuleHandleA(NULL), NULL);
    if (!hwnd) {
        free(start_w);
        free((void*)g_wv2_poll_url_w);
        g_wv2_poll_url_w = NULL;
        snprintf(g_wv2_error, sizeof(g_wv2_error), "could not create the login window");
        return 3;
    }
    g_wv2_hwnd = hwnd;
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    UpdateWindow(hwnd);

    /* per-user data folder so repeated logins keep the session */
    char udf[1024];
    udf[0] = 0;
    DWORD un = GetEnvironmentVariableA("LOCALAPPDATA", udf, (DWORD)sizeof(udf) - 1);
    if (un > 0 && un < sizeof(udf) - 32) {
        strncat(udf, "\\Wingram\\WebView2", sizeof(udf) - strlen(udf) - 1);
    } else {
        snprintf(udf, sizeof(udf), "%s", "WingramWebView2Data");
    }
    unsigned short* udf_w = still_wide(udf);
    if (!udf_w) udf_w = still_wide("");
    long hr = create_env(NULL, (const unsigned short*)udf_w, NULL, (void*)&g_wv2_env_handler);
    free(udf_w);
    if (hr != 0) {
        snprintf(g_wv2_error, sizeof(g_wv2_error),
                 "could not start the webview environment (0x%08lx)", (unsigned long)hr);
        g_wv2_status = 3;
    }

    MSG msg;
    while (g_wv2_status == 0 && GetMessageA(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }

    /* cleanup: teardown already ran from WM_CLOSE (it must happen before
       the window dies); this covers the paths that never went through
       WM_CLOSE (env-create failure and friends) */
    wv2_begin_teardown();
    /* release AFTER the pump so an in-flight GetCookies completion never
       sees a cookie manager we already freed */
    if (g_wv2_pending_cm) {
        wv2_release(g_wv2_pending_cm);
        g_wv2_pending_cm = NULL;
    }
    if (g_wv2_web2_obj) wv2_release(g_wv2_web2_obj);
    if (g_wv2_web_obj) wv2_release(g_wv2_web_obj);
    if (g_wv2_ctrl_obj) wv2_release(g_wv2_ctrl_obj);
    if (g_wv2_env_obj) wv2_release(g_wv2_env_obj);
    wv2_trace("objects released");
    if (IsWindow(g_wv2_hwnd)) DestroyWindow(g_wv2_hwnd);
    g_wv2_hwnd = NULL;
    free(start_w);
    g_wv2_start_url_w = NULL;
    free((void*)g_wv2_poll_url_w);
    g_wv2_poll_url_w = NULL;
    g_wv2_web2_obj = NULL;

    int result = g_wv2_status;
    if (result == 0) result = 2;
    return (int64_t)result;
}

#else /* !_WIN32: graceful no-op backends so the same program typechecks and runs everywhere */

static char* still_dup_cstr(const char* s) {
    size_t n = strlen(s);
    char* p = (char*)malloc(n + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, s, n + 1);
    return p;
}

int64_t still_net_available(void) { return 0; }
const char* still_net_last_error(void) { return "net backend requires Windows (WinHTTP)"; }
int64_t still_net_request(const char* method, const char* url, const char* headers,
                          const char* body, int64_t timeout_ms) {
    (void)method; (void)url; (void)headers; (void)body; (void)timeout_ms;
    return -1;
}
int64_t still_net_status(int64_t id) { (void)id; return 0; }
const char* still_net_header(int64_t id, const char* name) { (void)id; (void)name; return ""; }
const char* still_net_body(int64_t id) { (void)id; return ""; }
void still_net_free(int64_t id) { (void)id; }

int64_t still_gui_available(void) { return 0; }
int64_t still_gui_open_window(const char* title, int64_t w, int64_t h) {
    (void)title; (void)w; (void)h;
    return 0;
}
int64_t still_gui_window_open(int64_t handle) { (void)handle; return 0; }
void still_gui_close_window(int64_t handle) { (void)handle; }
void still_gui_set_title(int64_t handle, const char* title) { (void)handle; (void)title; }
void still_gui_set_bg(int64_t handle, int64_t rgb) { (void)handle; (void)rgb; }
int64_t still_gui_begin_frame(int64_t handle) { (void)handle; return 0; }
void still_gui_end_frame(int64_t handle) { (void)handle; }
int64_t still_gui_width(int64_t handle) { (void)handle; return 0; }
int64_t still_gui_height(int64_t handle) { (void)handle; return 0; }
int64_t still_gui_mouse_x(int64_t handle) { (void)handle; return -1; }
int64_t still_gui_mouse_y(int64_t handle) { (void)handle; return -1; }
int64_t still_gui_mouse_down(int64_t handle) { (void)handle; return 0; }
int64_t still_gui_mouse_clicked(int64_t handle) { (void)handle; return 0; }
int64_t still_gui_wheel_delta(int64_t handle) { (void)handle; return 0; }
const char* still_gui_text_input(int64_t handle) { (void)handle; return ""; }
int64_t still_gui_key(int64_t handle, int64_t vk) { (void)handle; (void)vk; return 0; }
int64_t still_gui_key_pressed(int64_t handle, int64_t vk) { (void)handle; (void)vk; return 0; }
void still_gui_text(int64_t handle, double x, double y, const char* text, int64_t rgb, double size) {
    (void)handle; (void)x; (void)y; (void)text; (void)rgb; (void)size;
}
void still_gui_text_bold(int64_t handle, double x, double y, const char* text, int64_t rgb, double size) {
    (void)handle; (void)x; (void)y; (void)text; (void)rgb; (void)size;
}
int64_t still_gui_text_width(int64_t handle, const char* text, double size) {
    (void)handle; (void)text; (void)size; return 0;
}
void still_gui_fill_rect(int64_t handle, double x, double y, double w, double h, int64_t rgb) {
    (void)handle; (void)x; (void)y; (void)w; (void)h; (void)rgb;
}
void still_gui_fill_round(int64_t handle, double x, double y, double w, double h, double rad, int64_t rgb) {
    (void)handle; (void)x; (void)y; (void)w; (void)h; (void)rad; (void)rgb;
}
void still_gui_fill_circle(int64_t handle, double cx, double cy, double rad, int64_t rgb) {
    (void)handle; (void)cx; (void)cy; (void)rad; (void)rgb;
}
void still_gui_stroke_circle(int64_t handle, double cx, double cy, double rad, double width, int64_t rgb) {
    (void)handle; (void)cx; (void)cy; (void)rad; (void)width; (void)rgb;
}
void still_gui_clip_push(int64_t handle, double x, double y, double w, double h) {
    (void)handle; (void)x; (void)y; (void)w; (void)h;
}
void still_gui_clip_pop(int64_t handle) { (void)handle; }
void still_gui_draw_image(int64_t handle, int64_t img, double x, double y, double w, double h) {
    (void)handle; (void)img; (void)x; (void)y; (void)w; (void)h;
}

int64_t still_audio_available(void) { return 0; }
void still_audio_stop(void) {}
int64_t still_audio_play_file(const char* path, int64_t loop) {
    (void)path; (void)loop;
    return -1;
}
int64_t still_audio_play_melody(const char* spec, int64_t loop) {
    (void)spec; (void)loop;
    return -1;
}

int64_t still_media_available(void) { return 0; }
int64_t still_media_load(const char* path) { (void)path; return 0; }
void still_media_free(int64_t img) { (void)img; }
int64_t still_media_width(int64_t img) { (void)img; return 0; }
int64_t still_media_height(int64_t img) { (void)img; return 0; }


int64_t still_net_save_body(int64_t id, const char* path) { (void)id; (void)path; return 0; }

int64_t still_webview_available(void) { return 0; }
const char* still_webview_runtime_version(void) { return ""; }
const char* still_webview_last_error(void) { return "the webview backend requires Windows"; }
int64_t still_webview_login(const char* start_url, const char* poll_url, const char* wanted, int64_t timeout_ms) {
    (void)start_url; (void)poll_url; (void)wanted; (void)timeout_ms;
    return 3;
}
const char* still_webview_cookie(const char* name) { (void)name; return ""; }
int64_t still_webview_poll_count(void) { return 0; }

int64_t still_gui_dpi(int64_t handle) { (void)handle; return 96; }
int64_t still_gui_system_dpi(void) { return 96; }
void still_gui_set_scale(int64_t handle, double scale) { (void)handle; (void)scale; }

int64_t still_media_video_available(void) { return 0; }
int64_t still_media_video_play(const char* path, int64_t hwnd) {
    (void)path; (void)hwnd;
    return 0;
}
void still_media_video_rect(int64_t gui_handle, double x, double y, double w, double h) {
    (void)gui_handle; (void)x; (void)y; (void)w; (void)h;
}
int64_t still_media_video_state(void) { return 0; }
void still_media_video_rewind(void) {}
void still_media_video_pause(void) {}
void still_media_video_resume(void) {}
void still_media_video_stop(void) {}

#endif /* _WIN32 */

/* Console re-attach: compiled programs default to the Windows subsystem
   (no terminal window behind the GUI), but CI and users pass flags like
   --selftest — re-attach to the parent console so print output is visible.
   Only re-points streams that have NO usable handle: when the caller
   redirected stdout/stderr to a file or pipe (CI probes), those handles
   win — hijacking them to CONOUT$ silently swallowed all probe output. */
void still_cli_console_init(int argc, char** argv) {
#ifdef _WIN32
    int want = 0;
    for (int i = 1; i < argc; i++) {
        if (argv[i] && argv[i][0] == '-' && argv[i][1] == '-') { want = 1; break; }
    }
    if (!want) return;

    /* is a std stream usable? console (FILE_TYPE_CHAR), file (DISK) or
       pipe (PIPE) all count; only a missing handle needs the attach */
    HANDLE so = GetStdHandle(STD_OUTPUT_HANDLE);
    HANDLE se = GetStdHandle(STD_ERROR_HANDLE);
    int need_out = (so == NULL || so == INVALID_HANDLE_VALUE ||
                    GetFileType(so) == FILE_TYPE_UNKNOWN);
    int need_err = (se == NULL || se == INVALID_HANDLE_VALUE ||
                    GetFileType(se) == FILE_TYPE_UNKNOWN);
    if (!need_out && !need_err) return;

    typedef int (__stdcall *AttachFn)(unsigned long);
    AttachFn attach = (AttachFn)GetProcAddress(GetModuleHandleA("kernel32.dll"), "AttachConsole");
    if (!attach || !attach((unsigned long)0xFFFFFFFFUL /*ATTACH_PARENT_PROCESS*/)) return;
    if (need_out) {
        FILE* f1 = freopen("CONOUT$", "w", stdout);
        (void)f1;
    }
    if (need_err) {
        FILE* f2 = freopen("CONOUT$", "w", stderr);
        (void)f2;
    }
#else
    (void)argc; (void)argv;
#endif
}

/* ------------------------------------------------------- crash logger
   GUI apps die without a trace when they fault. This writes a small
   wingram-crash.log next to the executable (code, address, module and a
   raw return-address trail) so user reports pinpoint the faulting spot.
   Safe on every platform: a no-op outside Windows. */
#ifdef _WIN32
static LONG __stdcall still_crash_filter(EXCEPTION_POINTERS* ep) {
    char path[1024];
    DWORD n = GetModuleFileNameA(NULL, path, (DWORD)sizeof(path) - 1);
    if (n > 0 && n < sizeof(path)) {
        path[n] = 0;
        char* slash = strrchr(path, '\\');
        if (slash) *(slash + 1) = 0;
        strncat(path, "wingram-crash.log", sizeof(path) - strlen(path) - 1);
        FILE* f = fopen(path, "a");
        if (f) {
            fprintf(f, "crash: exception 0x%08lX at %p\n",
                    ep && ep->ExceptionRecord ? (unsigned long)ep->ExceptionRecord->ExceptionCode : 0UL,
                    ep && ep->ExceptionRecord ? ep->ExceptionRecord->ExceptionAddress : NULL);
            if (ep && ep->ContextRecord) {
#if defined(_M_X64) || defined(__x86_64__)
                fprintf(f, "regs: rip=%p rsp=%p rbp=%p\n",
                        ep->ContextRecord->Rip, ep->ContextRecord->Rsp, ep->ContextRecord->Rbp);
                fprintf(f, "args: rax=%p rcx=%p rdx=%p r8=%p r9=%p\n",
                        ep->ContextRecord->Rax, ep->ContextRecord->Rcx,
                        ep->ContextRecord->Rdx, ep->ContextRecord->R8,
                        ep->ContextRecord->R9);
#endif
            }
            void* frames[24];
            USHORT count = 0;
            {
                /* resolved dynamically: some import libraries (tcc's) lack
                   CaptureStackBackTrace; kernel32 usually forwards it to
                   ntdll!RtlCaptureStackBackTrace */
                typedef unsigned short (WINAPI *CaptureTraceFn)(unsigned long, unsigned long, void**, unsigned long*);
                CaptureTraceFn cap = (CaptureTraceFn)GetProcAddress(GetModuleHandleA("kernel32.dll"), "CaptureStackBackTrace");
                if (!cap) cap = (CaptureTraceFn)GetProcAddress(GetModuleHandleA("ntdll.dll"), "RtlCaptureStackBackTrace");
                if (cap) count = cap(0, 24, frames, NULL);
            }
            fprintf(f, "stack (%u frames):\n", (unsigned)count);
            for (USHORT i = 0; i < count; i++) {
                HMODULE m = NULL;
                if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                                       GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                                       (LPCSTR)frames[i], &m) && m) {
                    char mname[256] = "?";
                    GetModuleFileNameA(m, mname, sizeof(mname) - 1);
                    const char* base = strrchr(mname, '\\');
                    base = base ? base + 1 : mname;
                    fprintf(f, "  %02u %p %s+0x%zX\n", (unsigned)i, frames[i], base,
                            (size_t)((char*)frames[i] - (char*)m));
                } else {
                    fprintf(f, "  %02u %p\n", (unsigned)i, frames[i]);
                }
            }
            fprintf(f, "version: still runtime crash logger\n\n");
            fclose(f);
        }
    }
    return EXCEPTION_CONTINUE_SEARCH;
}
void still_install_crash_logger(void) {
    SetUnhandledExceptionFilter(still_crash_filter);
}
#else
void still_install_crash_logger(void) {}
#endif

/* ---------------------------------------------------------------- filesystem helpers (v0.6.1) */
/* Directory listing and existence probe — universal, no platform backend
   needed. read_dir returns file/subdirectory NAMES (not paths) sorted by the
   OS order; it returns an empty list when the path cannot be opened, so
   programs can treat a missing folder as "no mods installed". */

still_list_s* still_read_dir(still_str* path) {
    still_list_s* out = still_list_s_alloc(8);
    char* p = (char*)malloc((size_t)path->len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, path->bytes, (size_t)path->len);
    p[path->len] = 0;
#ifdef _WIN32
    char pattern[1024];
    snprintf(pattern, sizeof(pattern), "%s\\*", p);
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA(pattern, &fd);
    if (h != INVALID_HANDLE_VALUE) {
        do {
            const char* name = fd.cFileName;
            if (name[0] == '.' && (name[1] == 0 || (name[1] == '.' && name[2] == 0))) continue;
            still_str* s = still_str_new(name, (int64_t)strlen(name));
            still_list_s_push(out, s);
        } while (FindNextFileA(h, &fd));
        FindClose(h);
    }
#else
    DIR* d = opendir(p);
    if (d) {
        struct dirent* e;
        while ((e = readdir(d)) != NULL) {
            const char* name = e->d_name;
            if (name[0] == '.' && (name[1] == 0 || (name[1] == '.' && name[2] == 0))) continue;
            still_str* s = still_str_new(name, (int64_t)strlen(name));
            still_list_s_push(out, s);
        }
        closedir(d);
    }
#endif
    free(p);
    return out;
}

/* Create a directory (parents created one level at a time, like mkdir -p on
   the leaf only). Returns 1 when the directory exists afterwards. */
int64_t still_dir_make(const char* path) {
    if (!path) return 0;
    size_t plen = strlen(path);
    char* p = (char*)malloc(plen + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, path, plen + 1);
    int ok = 0;
    /* walk the path creating each level (works for "cache/img" style paths) */
    for (char* q = p + 1; ; q++) {
        if (*q == '/' || *q == '\\' || *q == 0) {
            char saved = *q;
            *q = 0;
#ifdef _WIN32
            if (strlen(p) > 0 && CreateDirectoryA(p, NULL)) ok = 1;
            else if (GetFileAttributesA(p) != INVALID_FILE_ATTRIBUTES) ok = 1;
#else
            if (mkdir(p, 0755) == 0) ok = 1;
            else if (access(p, F_OK) == 0) ok = 1;
#endif
            *q = saved;
            if (*q == 0) break;
        }
    }
    free(p);
    return ok ? 1 : 0;
}

int64_t still_file_exists(still_str* path) {
    char* p = (char*)malloc((size_t)path->len + 1);
    if (!p) still_panic("memory", 0, "out of memory");
    memcpy(p, path->bytes, (size_t)path->len);
    p[path->len] = 0;
#ifdef _WIN32
    DWORD attrs = GetFileAttributesA(p);
    free(p);
    return (attrs != INVALID_FILE_ATTRIBUTES) ? 1 : 0;
#else
    struct stat st;
    int r = stat(p, &st);
    free(p);
    return (r == 0) ? 1 : 0;
#endif
}
