#ifndef STILL_RT_H
#define STILL_RT_H

#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _MSC_VER
static inline int still_msvc_add_ovf(int64_t a, int64_t b, int64_t* r) {
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) return 1;
    *r = a + b;
    return 0;
}
static inline int still_msvc_sub_ovf(int64_t a, int64_t b, int64_t* r) {
    if ((b < 0 && a > INT64_MAX + b) || (b > 0 && a < INT64_MIN + b)) return 1;
    *r = a - b;
    return 0;
}
static inline int still_msvc_mul_ovf(int64_t a, int64_t b, int64_t* r) {
    if (a == 0 || b == 0) { *r = 0; return 0; }
    if (a == INT64_MIN && b == -1) return 1;
    int64_t x = (int64_t)((uint64_t)a * (uint64_t)b);
    if (x / b != a) return 1;
    *r = x;
    return 0;
}
#define __builtin_add_overflow(a, b, r) still_msvc_add_ovf((a), (b), (r))
#define __builtin_sub_overflow(a, b, r) still_msvc_sub_ovf((a), (b), (r))
#define __builtin_mul_overflow(a, b, r) still_msvc_mul_ovf((a), (b), (r))
#endif

typedef struct {
    int64_t rc;
    void (*destroy)(void*);
} still_hdr;

typedef struct {
    still_hdr hdr;
    int64_t len;
    int64_t cap;
    char bytes[];
} still_str;

typedef struct {
    still_hdr hdr;
    int64_t len;
    int64_t cap;
    int64_t* data;
} still_list_i;

typedef struct {
    still_hdr hdr;
    int64_t len;
    int64_t cap;
    still_str** data;
} still_list_s;

void still_panic(const char* file, int64_t line, const char* msg);
void still_check_fail(const char* file, int64_t line, const char* msg);
void still_fail(still_str* msg, const char* file, int64_t line);
void still_exit(int64_t code);

static inline void* still_retain(void* p) {
    if (p) {
        ((still_hdr*)p)->rc++;
    }
    return p;
}

static inline void still_release(void* p) {
    if (p) {
        still_hdr* h = (still_hdr*)p;
        if (--h->rc == 0) {
            h->destroy(p);
        }
    }
}

still_str* still_str_new(const char* s, int64_t len);
void still_free_str(void* p);
void still_fmt_double(char* buf, double v);
void still_bp_check(const char* file, int64_t line);
still_str* still_str_from_owned(char* buf, int64_t len);
still_str* still_str_cat(still_str* a, still_str* b);
bool still_str_eq(still_str* a, still_str* b);
int64_t still_str_cmp(still_str* a, still_str* b);
still_str* still_str_upper(still_str* a);
still_str* still_str_lower(still_str* a);
still_str* still_str_trim(still_str* a);
bool still_str_contains(still_str* a, still_str* b);
bool still_str_starts(still_str* a, still_str* b);
bool still_str_ends(still_str* a, still_str* b);
still_str* still_str_slice(still_str* a, int64_t start, int64_t end);
still_str* still_str_index(still_str* a, int64_t i, const char* file, int64_t line);
still_str* still_str_from_int(int64_t v);
still_str* still_str_from_float(double v);
still_str* still_str_from_bool(bool v);
int64_t still_str_to_int(still_str* s, const char* file, int64_t line);
double still_str_to_float(still_str* s, const char* file, int64_t line);
int64_t still_ord(still_str* s, const char* file, int64_t line);
still_str* still_chr(int64_t c, const char* file, int64_t line);

still_list_i* still_range(int64_t n);
still_list_i* still_range2(int64_t a, int64_t b);
still_list_i* still_range3(int64_t a, int64_t b, int64_t step);

void still_list_i_push(still_list_i* l, int64_t v);
int64_t still_list_i_get(still_list_i* l, int64_t i, const char* file, int64_t line);
void still_list_i_set(still_list_i* l, int64_t i, int64_t v, const char* file, int64_t line);
still_list_i* still_list_i_concat(still_list_i* a, still_list_i* b);
bool still_list_i_eq(still_list_i* a, still_list_i* b);
bool still_list_i_contains(still_list_i* l, int64_t v);
still_list_i* still_list_i_slice(still_list_i* l, int64_t a, int64_t b);
still_list_i* still_list_i_reversed(still_list_i* l);
void still_list_i_clear(still_list_i* l);
int64_t still_list_i_pop(still_list_i* l, const char* file, int64_t line);
still_list_i* still_list_i_alloc(int64_t cap);

void still_list_s_push(still_list_s* l, still_str* v);
still_str* still_list_s_get(still_list_s* l, int64_t i, const char* file, int64_t line);
void still_list_s_set(still_list_s* l, int64_t i, still_str* v, const char* file, int64_t line);
still_list_s* still_list_s_concat(still_list_s* a, still_list_s* b);
bool still_list_s_eq(still_list_s* a, still_list_s* b);
bool still_list_s_contains(still_list_s* l, still_str* v);
still_list_s* still_list_s_slice(still_list_s* l, int64_t a, int64_t b);
still_list_s* still_list_s_reversed(still_list_s* l);
void still_list_s_clear(still_list_s* l);
still_str* still_list_s_pop(still_list_s* l, const char* file, int64_t line);
still_list_s* still_list_s_alloc(int64_t cap);

int64_t still_ipow(int64_t a, int64_t b, const char* file, int64_t line);
int64_t still_iabs(int64_t a);
double still_floordivf(double a, double b);
double still_fmodf_(double a, double b);

void still_print_str(still_str* s);
void still_print_int(int64_t v);
void still_print_float(double v);
void still_print_bool(bool v);
void still_print_none(void);
void still_print_sep(void);
void still_print_nl(void);

still_str* still_input(still_str* prompt);
double still_clock(void);
int64_t still_rand(void);
int64_t still_rand_bound(int64_t n);
void still_seed(int64_t s);

still_str* still_read_file(still_str* path);
int64_t still_write_file(still_str* path, still_str* data);
still_str* still_get_env(still_str* name);
void still_sleep_ms(int64_t ms);


#ifdef STIL_DEBUG
static inline int64_t still_add(int64_t a, int64_t b, const char* file, int64_t line) {
    int64_t r;
    if (__builtin_add_overflow(a, b, &r)) still_panic(file, line, "number too large (integer overflow)");
    return r;
}
static inline int64_t still_sub(int64_t a, int64_t b, const char* file, int64_t line) {
    int64_t r;
    if (__builtin_sub_overflow(a, b, &r)) still_panic(file, line, "number too small (integer overflow)");
    return r;
}
static inline int64_t still_mul(int64_t a, int64_t b, const char* file, int64_t line) {
    int64_t r;
    if (__builtin_mul_overflow(a, b, &r)) still_panic(file, line, "number too large (integer overflow)");
    return r;
}
#else
static inline int64_t still_add(int64_t a, int64_t b, const char* file, int64_t line) { (void)file; (void)line; return (int64_t)((uint64_t)a + (uint64_t)b); }
static inline int64_t still_sub(int64_t a, int64_t b, const char* file, int64_t line) { (void)file; (void)line; return (int64_t)((uint64_t)a - (uint64_t)b); }
static inline int64_t still_mul(int64_t a, int64_t b, const char* file, int64_t line) { (void)file; (void)line; return (int64_t)((uint64_t)a * (uint64_t)b); }
#endif
static inline int64_t still_idiv(int64_t a, int64_t b, const char* file, int64_t line) {
    if (b == 0) still_panic(file, line, "division by zero");
    if (a == INT64_MIN && b == -1) still_panic(file, line, "number too small (integer overflow)");
    return a / b;
}
static inline int64_t still_floordiv(int64_t a, int64_t b, const char* file, int64_t line) {
    if (b == 0) still_panic(file, line, "division by zero");
    if (a == INT64_MIN && b == -1) still_panic(file, line, "number too small (integer overflow)");
    int64_t q = a / b;
    if ((a % b != 0) && ((a < 0) != (b < 0))) q -= 1;
    return q;
}
static inline int64_t still_imod(int64_t a, int64_t b, const char* file, int64_t line) {
    if (b == 0) still_panic(file, line, "modulo by zero");
    if (a == INT64_MIN && b == -1) return 0;
    int64_t r = a % b;
    if (r != 0 && ((r < 0) != (b < 0))) r += b;
    return r;
}

void still_init(void);
int64_t still_soa_index(int64_t i, int64_t len, const char* file, int64_t line);
int64_t still_dict_next_cap(int64_t want);

uint64_t still_hash_i64(int64_t v);
uint64_t still_hash_str(still_str* s);
bool still_eq_i64(int64_t a, int64_t b);
bool still_eq_f64(double a, double b);
bool still_eq_bool(bool a, bool b);

#define STILL_DEFINE_LIST(T, TAG, RETAIN, RELEASE, EQ, CLONE) \
typedef struct { still_hdr hdr; int64_t len; int64_t cap; T* data; } still_list_##TAG; \
static void still_list_##TAG##_destroy(void* p) { \
    still_list_##TAG* l = (still_list_##TAG*)p; \
    for (int64_t _i = 0; _i < l->len; _i++) { T _v = l->data[_i]; (void)_v; RELEASE; } \
    free(l->data); \
    free(l); \
} \
static still_list_##TAG* still_list_##TAG##_alloc(int64_t cap) { \
    still_list_##TAG* l = (still_list_##TAG*)malloc(sizeof(still_list_##TAG)); \
    if (!l) still_panic("memory", 0, "out of memory"); \
    l->hdr.rc = 1; \
    l->hdr.destroy = still_list_##TAG##_destroy; \
    l->len = 0; \
    l->cap = cap > 0 ? cap : 8; \
    l->data = (T*)malloc((size_t)l->cap * sizeof(T)); \
    if (!l->data) still_panic("memory", 0, "out of memory"); \
    return l; \
} \
static void still_list_##TAG##_push(still_list_##TAG* l, T _v) { \
    if (l->len == l->cap) { \
        l->cap *= 2; \
        T* nd = (T*)realloc(l->data, (size_t)l->cap * sizeof(T)); \
        if (!nd) still_panic("memory", 0, "out of memory"); \
        l->data = nd; \
    } \
    l->data[l->len++] = _v; \
} \
static T still_list_##TAG##_get(still_list_##TAG* l, int64_t i, const char* file, int64_t line) { \
    if (i < 0) i += l->len; \
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range"); \
    T _v = l->data[i]; \
    return CLONE; \
} \
static void still_list_##TAG##_set(still_list_##TAG* l, int64_t i, T _v, const char* file, int64_t line) { \
    if (i < 0) i += l->len; \
    if (i < 0 || i >= l->len) still_panic(file, line, "index out of range"); \
    { T _o = l->data[i]; (void)_o; RELEASE; } \
    l->data[i] = _v; \
} \
static T still_list_##TAG##_pop(still_list_##TAG* l, const char* file, int64_t line) { \
    if (l->len == 0) still_panic(file, line, "cannot pop from an empty list"); \
    return l->data[--l->len]; \
} \
static void still_list_##TAG##_clear(still_list_##TAG* l) { \
    for (int64_t _i = 0; _i < l->len; _i++) { T _v = l->data[_i]; (void)_v; RELEASE; } \
    l->len = 0; \
} \
static still_list_##TAG* still_list_##TAG##_concat(still_list_##TAG* a, still_list_##TAG* b) { \
    still_list_##TAG* r = still_list_##TAG##_alloc(a->len + b->len); \
    for (int64_t _i = 0; _i < a->len; _i++) { T _v = a->data[_i]; (void)_v; still_list_##TAG##_push(r, RETAIN); } \
    for (int64_t _i = 0; _i < b->len; _i++) { T _v = b->data[_i]; (void)_v; still_list_##TAG##_push(r, RETAIN); } \
    return r; \
} \
static bool still_list_##TAG##_eq(still_list_##TAG* a, still_list_##TAG* b) { \
    if (a == b) return true; \
    if (!a || !b || a->len != b->len) return false; \
    for (int64_t _i = 0; _i < a->len; _i++) { if (!EQ(a->data[_i], b->data[_i])) return false; } \
    return true; \
} \
static bool still_list_##TAG##_contains(still_list_##TAG* l, T _v) { \
    for (int64_t _i = 0; _i < l->len; _i++) { if (EQ(l->data[_i], _v)) return true; } \
    return false; \
} \
static still_list_##TAG* still_list_##TAG##_slice(still_list_##TAG* l, int64_t a, int64_t b) { \
    if (a < 0) a += l->len; \
    if (b < 0) b += l->len; \
    if (a < 0) a = 0; \
    if (b > l->len) b = l->len; \
    if (b < a) b = a; \
    still_list_##TAG* r = still_list_##TAG##_alloc(b - a); \
    for (int64_t _i = a; _i < b; _i++) { T _v = l->data[_i]; (void)_v; still_list_##TAG##_push(r, RETAIN); } \
    return r; \
} \
static still_list_##TAG* still_list_##TAG##_reversed(still_list_##TAG* l) { \
    still_list_##TAG* r = still_list_##TAG##_alloc(l->len); \
    for (int64_t _i = l->len - 1; _i >= 0; _i--) { T _v = l->data[_i]; (void)_v; still_list_##TAG##_push(r, RETAIN); } \
    return r; \
}

#define STILL_DEFINE_DICT(CK, CV, KTAG, VTAG, HASHK, EQK, EQV, RELEASEK, RELEASEV, CLONEV) \
typedef struct { uint8_t state; CK key; CV val; } still_dslot_##KTAG##_##VTAG; \
typedef struct { still_hdr hdr; int64_t len; int64_t cap; int64_t used; still_dslot_##KTAG##_##VTAG* slots; } still_dict_##KTAG##_##VTAG; \
static void still_dict_##KTAG##_##VTAG##_destroy(void* p) { \
    still_dict_##KTAG##_##VTAG* d = (still_dict_##KTAG##_##VTAG*)p; \
    for (int64_t _i = 0; _i < d->cap; _i++) { \
        if (d->slots[_i].state == 1) { CK _k = d->slots[_i].key; (void)_k; CV _v = d->slots[_i].val; (void)_v; RELEASEK; RELEASEV; } \
    } \
    free(d->slots); \
    free(d); \
} \
static still_dict_##KTAG##_##VTAG* still_dict_##KTAG##_##VTAG##_alloc(void) { \
    still_dict_##KTAG##_##VTAG* d = (still_dict_##KTAG##_##VTAG*)malloc(sizeof(still_dict_##KTAG##_##VTAG)); \
    if (!d) still_panic("memory", 0, "out of memory"); \
    d->hdr.rc = 1; \
    d->hdr.destroy = still_dict_##KTAG##_##VTAG##_destroy; \
    d->len = 0; \
    d->cap = 8; \
    d->used = 0; \
    d->slots = (still_dslot_##KTAG##_##VTAG*)calloc(8, sizeof(still_dslot_##KTAG##_##VTAG)); \
    if (!d->slots) still_panic("memory", 0, "out of memory"); \
    return d; \
} \
static int64_t still_dict_##KTAG##_##VTAG##_find(still_dict_##KTAG##_##VTAG* d, CK _k) { \
    uint64_t _h = HASHK; \
    int64_t _i = (int64_t)(_h & (uint64_t)(d->cap - 1)); \
    while (d->slots[_i].state != 0) { \
        if (d->slots[_i].state == 1 && EQK(d->slots[_i].key, _k)) return _i; \
        _i = (_i + 1) & (d->cap - 1); \
    } \
    return -1; \
} \
static void still_dict_##KTAG##_##VTAG##_grow(still_dict_##KTAG##_##VTAG* d) { \
    int64_t _nc = d->cap * 2; \
    still_dslot_##KTAG##_##VTAG* ns = (still_dslot_##KTAG##_##VTAG*)calloc((size_t)_nc, sizeof(still_dslot_##KTAG##_##VTAG)); \
    if (!ns) still_panic("memory", 0, "out of memory"); \
    for (int64_t _j = 0; _j < d->cap; _j++) { \
        if (d->slots[_j].state == 1) { \
            CK _k = d->slots[_j].key; \
            uint64_t _h2 = HASHK; \
            int64_t _t = (int64_t)(_h2 & (uint64_t)(_nc - 1)); \
            while (ns[_t].state != 0) _t = (_t + 1) & (_nc - 1); \
            ns[_t] = d->slots[_j]; \
        } \
    } \
    free(d->slots); \
    d->slots = ns; \
    d->cap = _nc; \
    d->used = d->len; \
} \
static void still_dict_##KTAG##_##VTAG##_set(still_dict_##KTAG##_##VTAG* d, CK _k, CV _v) { \
    int64_t _i = still_dict_##KTAG##_##VTAG##_find(d, _k); \
    if (_i >= 0) { \
        CV _o = d->slots[_i].val; (void)_o; RELEASEV; \
        d->slots[_i].val = _v; \
        return; \
    } \
    if ((d->len + 1) * 10 >= d->cap * 7) { \
        still_dict_##KTAG##_##VTAG##_grow(d); \
    } \
    uint64_t _h = HASHK; \
    int64_t _t = (int64_t)(_h & (uint64_t)(d->cap - 1)); \
    while (d->slots[_t].state == 1) _t = (_t + 1) & (d->cap - 1); \
    if (d->slots[_t].state == 2) d->used--; \
    d->slots[_t].state = 1; \
    d->slots[_t].key = _k; \
    d->slots[_t].val = _v; \
    d->len++; \
    d->used++; \
} \
static CV still_dict_##KTAG##_##VTAG##_get(still_dict_##KTAG##_##VTAG* d, CK _k, const char* file, int64_t line) { \
    int64_t _i = still_dict_##KTAG##_##VTAG##_find(d, _k); \
    if (_i < 0) still_panic(file, line, "missing key in dict"); \
    CV _v = d->slots[_i].val; \
    return CLONEV; \
} \
static bool still_dict_##KTAG##_##VTAG##_has(still_dict_##KTAG##_##VTAG* d, CK _k) { \
    return still_dict_##KTAG##_##VTAG##_find(d, _k) >= 0; \
} \
static void still_dict_##KTAG##_##VTAG##_clear(still_dict_##KTAG##_##VTAG* d) { \
    for (int64_t _i = 0; _i < d->cap; _i++) { \
        if (d->slots[_i].state == 1) { CK _k = d->slots[_i].key; (void)_k; CV _v = d->slots[_i].val; (void)_v; RELEASEK; RELEASEV; } \
        d->slots[_i].state = 0; \
    } \
    d->len = 0; \
    d->used = 0; \
} \
static bool still_dict_##KTAG##_##VTAG##_eq(still_dict_##KTAG##_##VTAG* a, still_dict_##KTAG##_##VTAG* b) { \
    if (a == b) return true; \
    if (!a || !b || a->len != b->len) return false; \
    for (int64_t _i = 0; _i < a->cap; _i++) { \
        if (a->slots[_i].state == 1) { \
            int64_t _j = still_dict_##KTAG##_##VTAG##_find(b, a->slots[_i].key); \
            if (_j < 0) return false; \
            if (!EQV(a->slots[_i].val, b->slots[_j].val)) return false; \
        } \
    } \
    return true; \
}

#endif
still_list_s* still_str_split(still_str* s, still_str* sep);
bool still_ptr_eq(void* a, void* b);

typedef struct {
    char* buf;
    int64_t len;
    int64_t cap;
} still_sb;

still_sb* still_sb_new(void);
void still_sb_c(still_sb* sb, char c);
void still_sb_s(still_sb* sb, still_str* s);
void still_sb_raw(still_sb* sb, const char* s);
still_str* still_sb_finish(still_sb* sb);
void still_sb_add_int(still_sb* sb, int64_t v);
void still_sb_add_float(still_sb* sb, double v);
void still_sb_add_bool(still_sb* sb, bool v);
void still_sb_add_str(still_sb* sb, still_str* v);
void still_sb_add_none(still_sb* sb);
void still_tostr_add_soa_stub(still_sb* sb, void* p);
typedef struct { still_hdr hdr; int64_t len; } still_soa_hdr;
