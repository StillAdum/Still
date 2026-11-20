#include "still_rt.h"
#define STIL_DEBUG 1
#include <math.h>
/* extern "C" declarations (emitted by Still) */
typedef struct {
    bool ok;
    still_any* value;
    still_str* error;
} still_st_json_u003a__u003a_JsonResult;
static void still_st_json_u003a__u003a_JsonResult_drop(still_st_json_u003a__u003a_JsonResult* v) {
    { bool _v = v->ok; (void)_v; ((void)_v); }
    { still_any* _v = v->value; (void)_v; still_release(_v); }
    { still_str* _v = v->error; (void)_v; still_release(_v); }
}
static still_st_json_u003a__u003a_JsonResult still_st_json_u003a__u003a_JsonResult_clone(still_st_json_u003a__u003a_JsonResult* v) {
    still_st_json_u003a__u003a_JsonResult r = *v;
    { bool _v = r.ok; (void)_v; r.ok = _v; }
    { still_any* _v = r.value; (void)_v; r.value = (still_any*)still_retain(_v); }
    { still_str* _v = r.error; (void)_v; r.error = (still_str*)still_retain(_v); }
    return r;
}
static bool still_st_json_u003a__u003a_JsonResult_eq(still_st_json_u003a__u003a_JsonResult* a, still_st_json_u003a__u003a_JsonResult* b) {
    return true && still_eq_bool(a->ok, b->ok) && still_any_eq(a->value, b->value) && still_str_eq(a->error, b->error);
}
static bool still_st_json_u003a__u003a_JsonResult_eqv(still_st_json_u003a__u003a_JsonResult a, still_st_json_u003a__u003a_JsonResult b) { return still_st_json_u003a__u003a_JsonResult_eq(&a, &b); }

typedef struct {
    still_str* text;
    int64_t pos;
    still_str* err;
} still_st_json_u003a__u003a_JsonParser;
static void still_st_json_u003a__u003a_JsonParser_drop(still_st_json_u003a__u003a_JsonParser* v) {
    { still_str* _v = v->text; (void)_v; still_release(_v); }
    { int64_t _v = v->pos; (void)_v; ((void)_v); }
    { still_str* _v = v->err; (void)_v; still_release(_v); }
}
static still_st_json_u003a__u003a_JsonParser still_st_json_u003a__u003a_JsonParser_clone(still_st_json_u003a__u003a_JsonParser* v) {
    still_st_json_u003a__u003a_JsonParser r = *v;
    { still_str* _v = r.text; (void)_v; r.text = (still_str*)still_retain(_v); }
    { int64_t _v = r.pos; (void)_v; r.pos = _v; }
    { still_str* _v = r.err; (void)_v; r.err = (still_str*)still_retain(_v); }
    return r;
}
static bool still_st_json_u003a__u003a_JsonParser_eq(still_st_json_u003a__u003a_JsonParser* a, still_st_json_u003a__u003a_JsonParser* b) {
    return true && still_str_eq(a->text, b->text) && still_eq_i64(a->pos, b->pos) && still_str_eq(a->err, b->err);
}
static bool still_st_json_u003a__u003a_JsonParser_eqv(still_st_json_u003a__u003a_JsonParser a, still_st_json_u003a__u003a_JsonParser b) { return still_st_json_u003a__u003a_JsonParser_eq(&a, &b); }

typedef struct {
    still_list_s* parts;
} still_st_json_u003a__u003a_StringBuilder;
static void still_st_json_u003a__u003a_StringBuilder_drop(still_st_json_u003a__u003a_StringBuilder* v) {
    { still_list_s* _v = v->parts; (void)_v; still_release(_v); }
}
static still_st_json_u003a__u003a_StringBuilder still_st_json_u003a__u003a_StringBuilder_clone(still_st_json_u003a__u003a_StringBuilder* v) {
    still_st_json_u003a__u003a_StringBuilder r = *v;
    { still_list_s* _v = r.parts; (void)_v; r.parts = (still_list_s*)still_retain(_v); }
    return r;
}
static bool still_st_json_u003a__u003a_StringBuilder_eq(still_st_json_u003a__u003a_StringBuilder* a, still_st_json_u003a__u003a_StringBuilder* b) {
    return true && still_list_s_eq(a->parts, b->parts);
}
static bool still_st_json_u003a__u003a_StringBuilder_eqv(still_st_json_u003a__u003a_StringBuilder a, still_st_json_u003a__u003a_StringBuilder b) { return still_st_json_u003a__u003a_StringBuilder_eq(&a, &b); }

STILL_DEFINE_DICT(still_str*, still_any*, s, a, still_hash_str(_k), still_str_eq, still_any_eq, still_release(_k), still_release(_v), (still_any*)still_retain(_v))

typedef struct {
    still_str* f0;
    still_any* f1;
} still_tup_s_a;
static void still_tup_s_a_drop(still_tup_s_a* v) {
    { still_str* _v = v->f0; (void)_v; still_release(_v); }
    { still_any* _v = v->f1; (void)_v; still_release(_v); }
}
static still_tup_s_a still_tup_s_a_clone(still_tup_s_a* v) {
    still_tup_s_a r = *v;
    { still_str* _v = r.f0; (void)_v; r.f0 = (still_str*)still_retain(_v); }
    { still_any* _v = r.f1; (void)_v; r.f1 = (still_any*)still_retain(_v); }
    return r;
}
static bool still_tup_s_a_eq(still_tup_s_a* a, still_tup_s_a* b) {
    return true && still_str_eq(a->f0, b->f0) && still_any_eq(a->f1, b->f1);
}
static bool still_tup_s_a_eqv(still_tup_s_a a, still_tup_s_a b) { return still_tup_s_a_eq(&a, &b); }

STILL_DEFINE_LIST(still_tup_s_a, Ts_a, still_tup_s_a_clone(&_v), still_tup_s_a_drop(&_v), still_tup_s_a_eqv, still_tup_s_a_clone(&_v))

static still_list_s* still_dict_s_a_keys(still_dict_s_a* d) {
still_list_s* r = still_list_s_alloc(d->len);
for (int64_t _i = 0; _i < d->cap; _i++) {
if (d->slots[_i].state == 1) { still_str* _v = d->slots[_i].key; (void)_v; still_list_s_push(r, (still_str*)still_retain(_v)); }
}
return r;
}
static still_list_Ts_a* still_dict_s_a_pairs(still_dict_s_a* d) {
still_list_Ts_a* r = still_list_Ts_a_alloc(d->len);
for (int64_t _i = 0; _i < d->cap; _i++) {
if (d->slots[_i].state == 1) {
still_tup_s_a _p;
{ still_str* _v = d->slots[_i].key; (void)_v; _p.f0 = (still_str*)still_retain(_v); }
{ still_any* _v = d->slots[_i].val; (void)_v; _p.f1 = (still_any*)still_retain(_v); }
still_list_Ts_a_push(r, _p);
}
}
return r;
}

STILL_DEFINE_LIST(still_any*, a, (still_any*)still_retain(_v), still_release(_v), still_any_eq, (still_any*)still_retain(_v))

static still_st_json_u003a__u003a_JsonResult still_g_main__r = {0};
static still_any* still_g_main__v = NULL;
static still_any* still_g_main__xs = NULL;
static still_any* still_g_main__sub = NULL;
static still_st_json_u003a__u003a_JsonResult still_g_main__bad = {0};
static still_st_json_u003a__u003a_JsonResult still_g_main__esc = {0};
static still_st_json_u003a__u003a_JsonResult still_g_main__nums = {0};
static void still_mod_json__init(void);
static still_st_json_u003a__u003a_JsonResult still_json__parse(still_str* v0);
static still_any* still_json__JsonParser_parse_value(still_st_json_u003a__u003a_JsonParser* v0);
static void still_json__JsonParser_skip_ws(still_st_json_u003a__u003a_JsonParser* v0);
static bool still_json__JsonParser_at_end(still_st_json_u003a__u003a_JsonParser* v0);
static bool still_json__JsonParser_fail_at(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1);
static still_str* still_json__JsonParser_peek(still_st_json_u003a__u003a_JsonParser* v0);
static still_any* still_json__JsonParser_parse_object(still_st_json_u003a__u003a_JsonParser* v0);
static bool still_json__JsonParser_expect(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1);
static still_str* still_json__JsonParser_parse_string_raw(still_st_json_u003a__u003a_JsonParser* v0);
static still_str* still_json__JsonParser_peek_at(still_st_json_u003a__u003a_JsonParser* v0, int64_t v1);
static int64_t still_json__JsonParser_parse_hex4(still_st_json_u003a__u003a_JsonParser* v0);
static int64_t still_json__hex_digit(still_str* v0);
static still_str* still_json__JsonParser_utf8_encode(still_st_json_u003a__u003a_JsonParser* v0, int64_t v1);
static still_any* still_json__JsonParser_parse_array(still_st_json_u003a__u003a_JsonParser* v0);
static still_any* still_json__JsonParser_parse_literal(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1, still_any* v2);
static still_any* still_json__JsonParser_parse_number(still_st_json_u003a__u003a_JsonParser* v0);
static still_str* still_json__get_str(still_any* v0, still_str* v1, still_str* v2);
static still_any* still_json__get_any(still_any* v0, still_str* v1, still_any* v2);
static int64_t still_json__get_int(still_any* v0, still_str* v1, int64_t v2);
static double still_json__get_float(still_any* v0, still_str* v1, double v2);
static bool still_json__get_bool(still_any* v0, still_str* v1, bool v2);
static still_any* still_json__get_list(still_any* v0, still_str* v1, still_any* v2);
static still_any* still_json__get_dict(still_any* v0, still_str* v1, still_any* v2);
static still_str* still_json__stringify(still_any* v0);
static void still_json__write_value(still_st_json_u003a__u003a_StringBuilder v0, still_any* v1);
static void still_json__StringBuilder_append(still_st_json_u003a__u003a_StringBuilder* v0, still_str* v1);
static void still_json__write_string(still_st_json_u003a__u003a_StringBuilder v0, still_str* v1);
static still_str* still_json__hex4(int64_t v0);
static still_str* still_json__StringBuilder_to_string(still_st_json_u003a__u003a_StringBuilder* v0);
static bool still_json__valid(still_str* v0);
static still_str* still_json__stringify_pretty(still_any* v0, int64_t v1);
static still_any* still_json__get_at(still_any* v0, still_list_s* v1);
static void still_mod_json__init(void) {
    return;
}

static still_st_json_u003a__u003a_JsonResult still_json__parse(still_str* v0) {
#line 309 "json"
    still_str* t2 = still_str_new("", 0);
    still_st_json_u003a__u003a_JsonParser t1 = { .text = (still_str*)still_retain(v0), .pos = 0, .err = t2 };
    still_st_json_u003a__u003a_JsonParser v1 = t1;
    still_any* t3 = still_json__JsonParser_parse_value(&v1);
    still_any* v2 = t3;
#line 311 "json"
    still_bp_check(__FILE__, __LINE__);
#line 311 "json"
    still_str* t4 = still_str_new("", 0);
    if ((still_str_cmp(v1.err, t4) != 0)) {
#line 312 "json"
    still_bp_check(__FILE__, __LINE__);
#line 312 "json"
    still_st_json_u003a__u003a_JsonResult t5 = { .ok = false, .value = NULL, .error = (still_str*)still_retain(v1.err) };
    if (v2) still_release(v2);
    still_st_json_u003a__u003a_JsonParser_drop(&v1);
    if (v0) still_release(v0);
    if (t4) still_release(t4);
    return t5;
    }
#line 313 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&v1));
#line 314 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((!(still_json__JsonParser_at_end(&v1))))) {
#line 315 "json"
    still_bp_check(__FILE__, __LINE__);
#line 315 "json"
    still_str* t7 = still_str_new("JSON error at offset ", 21);
    still_str* t8 = still_str_from_int(v1.pos);
    still_str* t9 = still_str_cat(t7, t8);
    still_str* t10 = still_str_new(": unexpected trailing characters", 32);
    still_str* t11 = still_str_cat(t9, t10);
    still_st_json_u003a__u003a_JsonResult t6 = { .ok = false, .value = (still_any*)still_retain(v2), .error = t11 };
    if (v2) still_release(v2);
    still_st_json_u003a__u003a_JsonParser_drop(&v1);
    if (v0) still_release(v0);
    if (t10) still_release(t10);
    if (t9) still_release(t9);
    if (t8) still_release(t8);
    if (t7) still_release(t7);
    return t6;
    }
#line 316 "json"
    still_bp_check(__FILE__, __LINE__);
#line 316 "json"
    still_str* t13 = still_str_new("", 0);
    still_st_json_u003a__u003a_JsonResult t12 = { .ok = true, .value = (still_any*)still_retain(v2), .error = t13 };
    if (v2) still_release(v2);
    still_st_json_u003a__u003a_JsonParser_drop(&v1);
    if (v0) still_release(v0);
    return t12;
    if (v2) still_release(v2);
    still_st_json_u003a__u003a_JsonParser_drop(&v1);
    if (v0) still_release(v0);
    return (still_st_json_u003a__u003a_JsonResult){0};
}

static still_any* still_json__JsonParser_parse_value(still_st_json_u003a__u003a_JsonParser* v0) {
#line 58 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
#line 59 "json"
    still_bp_check(__FILE__, __LINE__);
    if ((still_json__JsonParser_at_end(&(*v0)))) {
#line 60 "json"
    still_bp_check(__FILE__, __LINE__);
#line 60 "json"
    still_str* t1 = still_str_new("unexpected end of input", 23);
    (void)(still_json__JsonParser_fail_at(&(*v0), t1));
#line 61 "json"
    still_bp_check(__FILE__, __LINE__);
    return NULL;
    }
#line 62 "json"
    still_str* t2 = still_json__JsonParser_peek(&(*v0));
    still_str* v1 = t2;
#line 63 "json"
    still_bp_check(__FILE__, __LINE__);
#line 63 "json"
    still_str* t3 = still_str_new("{", 1);
    if ((still_str_cmp(v1, t3) == 0)) {
#line 64 "json"
    still_bp_check(__FILE__, __LINE__);
#line 64 "json"
    still_any* t4 = still_json__JsonParser_parse_object(&(*v0));
    if (v1) still_release(v1);
    if (t3) still_release(t3);
    return t4;
    }
#line 65 "json"
    still_bp_check(__FILE__, __LINE__);
#line 65 "json"
    still_str* t5 = still_str_new("[", 1);
    if ((still_str_cmp(v1, t5) == 0)) {
#line 66 "json"
    still_bp_check(__FILE__, __LINE__);
#line 66 "json"
    still_any* t6 = still_json__JsonParser_parse_array(&(*v0));
    if (v1) still_release(v1);
    if (t5) still_release(t5);
    return t6;
    }
#line 67 "json"
    still_bp_check(__FILE__, __LINE__);
#line 67 "json"
    still_str* t7 = still_str_new("\"", 1);
    if ((still_str_cmp(v1, t7) == 0)) {
#line 68 "json"
    still_bp_check(__FILE__, __LINE__);
#line 68 "json"
    still_str* t8 = still_json__JsonParser_parse_string_raw(&(*v0));
    still_any* t9 = still_any_new_s(t8);
    still_release(t8);
    if (v1) still_release(v1);
    if (t7) still_release(t7);
    return t9;
    }
#line 69 "json"
    still_bp_check(__FILE__, __LINE__);
#line 69 "json"
    still_str* t10 = still_str_new("t", 1);
    if ((still_str_cmp(v1, t10) == 0)) {
#line 70 "json"
    still_bp_check(__FILE__, __LINE__);
#line 70 "json"
    still_str* t11 = still_str_new("true", 4);
    still_any* t12 = still_json__JsonParser_parse_literal(&(*v0), t11, still_any_new_b((int64_t)(true)));
    if (v1) still_release(v1);
    if (t10) still_release(t10);
    return t12;
    }
#line 71 "json"
    still_bp_check(__FILE__, __LINE__);
#line 71 "json"
    still_str* t13 = still_str_new("f", 1);
    if ((still_str_cmp(v1, t13) == 0)) {
#line 72 "json"
    still_bp_check(__FILE__, __LINE__);
#line 72 "json"
    still_str* t14 = still_str_new("false", 5);
    still_any* t15 = still_json__JsonParser_parse_literal(&(*v0), t14, still_any_new_b((int64_t)(false)));
    if (v1) still_release(v1);
    if (t13) still_release(t13);
    return t15;
    }
#line 73 "json"
    still_bp_check(__FILE__, __LINE__);
#line 73 "json"
    still_str* t16 = still_str_new("n", 1);
    if ((still_str_cmp(v1, t16) == 0)) {
#line 74 "json"
    still_bp_check(__FILE__, __LINE__);
#line 74 "json"
    still_str* t17 = still_str_new("null", 4);
    still_any* t18 = still_json__JsonParser_parse_literal(&(*v0), t17, NULL);
    if (v1) still_release(v1);
    if (t16) still_release(t16);
    return t18;
    }
#line 75 "json"
    still_bp_check(__FILE__, __LINE__);
#line 75 "json"
    still_str* t19 = still_str_new("-", 1);
    still_str* t20 = still_str_new("0", 1);
    still_str* t21 = still_str_new("9", 1);
    bool t22;
    if ((still_str_cmp(v1, t20) >= 0)) {
    t22 = still_str_cmp(v1, t21) <= 0;
    } else {
    t22 = still_str_cmp(v1, t20) >= 0;
    }
    bool t23;
    if ((still_str_cmp(v1, t19) == 0)) {
    t23 = still_str_cmp(v1, t19) == 0;
    } else {
    t23 = t22;
    }
    if ((t23)) {
#line 76 "json"
    still_bp_check(__FILE__, __LINE__);
#line 76 "json"
    still_any* t24 = still_json__JsonParser_parse_number(&(*v0));
    if (v1) still_release(v1);
    if (t21) still_release(t21);
    if (t20) still_release(t20);
    if (t19) still_release(t19);
    return t24;
    }
#line 77 "json"
    still_bp_check(__FILE__, __LINE__);
#line 77 "json"
    still_str* t25 = still_str_new("unexpected character", 20);
    (void)(still_json__JsonParser_fail_at(&(*v0), t25));
#line 78 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v1) still_release(v1);
    return NULL;
    if (v1) still_release(v1);
    return NULL;
}

static void still_json__JsonParser_skip_ws(still_st_json_u003a__u003a_JsonParser* v0) {
#line 38 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
#line 38 "json"
    if (!((((*v0).pos < (*v0).text->len)))) {
    break;
    }
    still_str* t1 = still_str_index((*v0).text, (*v0).pos, __FILE__, __LINE__);
    still_str* v1 = t1;
#line 40 "json"
    still_bp_check(__FILE__, __LINE__);
#line 40 "json"
    still_str* t2 = still_str_new(" ", 1);
    still_str* t3 = still_str_new("\t", 1);
    bool t4;
    if ((still_str_cmp(v1, t2) == 0)) {
    t4 = still_str_cmp(v1, t2) == 0;
    } else {
    t4 = still_str_cmp(v1, t3) == 0;
    }
    still_str* t5 = still_str_new("\n", 1);
    bool t6;
    if ((t4)) {
    t6 = t4;
    } else {
    t6 = still_str_cmp(v1, t5) == 0;
    }
    still_str* t7 = still_str_new("\r", 1);
    bool t8;
    if ((t6)) {
    t8 = t6;
    } else {
    t8 = still_str_cmp(v1, t7) == 0;
    }
    if ((t8)) {
#line 41 "json"
    still_bp_check(__FILE__, __LINE__);
#line 41 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    } else {
    if (t7) still_release(t7);
    if (t5) still_release(t5);
    if (t3) still_release(t3);
    if (t2) still_release(t2);
    if (v1) still_release(v1);
    break;
    }
    if (v1) still_release(v1);
    }
    return;
}

static bool still_json__JsonParser_at_end(still_st_json_u003a__u003a_JsonParser* v0) {
#line 25 "json"
    still_bp_check(__FILE__, __LINE__);
#line 25 "json"
    bool t1 = ((*v0).pos >= (*v0).text->len);
    return t1;
    return false;
}

static bool still_json__JsonParser_fail_at(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1) {
#line 46 "json"
    still_bp_check(__FILE__, __LINE__);
#line 46 "json"
    still_str* t1 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t1) == 0)) {
#line 47 "json"
    still_bp_check(__FILE__, __LINE__);
#line 47 "json"
    still_str* t2 = still_str_new("JSON error at offset ", 21);
    still_str* t3 = still_str_from_int((*v0).pos);
    still_str* t4 = still_str_cat(t2, t3);
    still_str* t5 = still_str_new(": ", 2);
    still_str* t6 = still_str_cat(t4, t5);
    still_str* t7 = (still_str*)still_retain(v1);
    still_str* t8 = still_str_cat(t6, t7);
    if ((*v0).err) still_release((*v0).err);
    (*v0).err = t8;
    if (t7) still_release(t7);
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    if (t4) still_release(t4);
    if (t3) still_release(t3);
    if (t2) still_release(t2);
    }
    if (t1) still_release(t1);
#line 48 "json"
    still_bp_check(__FILE__, __LINE__);
    bool t9 = false;
    if (v1) still_release(v1);
    return t9;
    if (v1) still_release(v1);
    return false;
}

static still_str* still_json__JsonParser_peek(still_st_json_u003a__u003a_JsonParser* v0) {
#line 28 "json"
    still_bp_check(__FILE__, __LINE__);
#line 28 "json"
    if ((((*v0).pos >= (*v0).text->len))) {
#line 29 "json"
    still_bp_check(__FILE__, __LINE__);
#line 29 "json"
    still_str* t1 = still_str_new("", 0);
    return t1;
    }
#line 30 "json"
    still_bp_check(__FILE__, __LINE__);
#line 30 "json"
    still_str* t2 = still_str_index((*v0).text, (*v0).pos, __FILE__, __LINE__);
    return t2;
    return NULL;
}

static still_any* still_json__JsonParser_parse_object(still_st_json_u003a__u003a_JsonParser* v0) {
#line 89 "json"
    still_bp_check(__FILE__, __LINE__);
#line 89 "json"
    still_str* t1 = still_str_new("{", 1);
    if (((!(still_json__JsonParser_expect(&(*v0), t1))))) {
#line 90 "json"
    still_bp_check(__FILE__, __LINE__);
    return NULL;
    }
#line 91 "json"
    still_dict_s_a* t2 = still_dict_s_a_alloc();
    still_dict_s_a* v1 = t2;
#line 92 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
#line 93 "json"
    still_bp_check(__FILE__, __LINE__);
#line 93 "json"
    still_str* t3 = still_json__JsonParser_peek(&(*v0));
    still_str* t4 = still_str_new("}", 1);
    if ((still_str_cmp(t3, t4) == 0)) {
#line 94 "json"
    still_bp_check(__FILE__, __LINE__);
#line 94 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 95 "json"
    still_bp_check(__FILE__, __LINE__);
    still_dict_s_a* t5 = (still_dict_s_a*)still_retain(v1);
    still_any* t6 = still_any_new_p((void*)(t5), ((int64_t)6386532));
    still_release(t5);
    if (v1) still_release(v1);
    if (t4) still_release(t4);
    if (t3) still_release(t3);
    return t6;
    }
#line 96 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!((true))) {
    break;
    }
#line 97 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
#line 98 "json"
    still_bp_check(__FILE__, __LINE__);
#line 98 "json"
    still_str* t7 = still_json__JsonParser_peek(&(*v0));
    still_str* t8 = still_str_new("\"", 1);
    if ((still_str_cmp(t7, t8) != 0)) {
#line 99 "json"
    still_bp_check(__FILE__, __LINE__);
#line 99 "json"
    still_str* t9 = still_str_new("object keys must be strings", 27);
    (void)(still_json__JsonParser_fail_at(&(*v0), t9));
#line 100 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v1) still_release(v1);
    if (t8) still_release(t8);
    if (t7) still_release(t7);
    return NULL;
    }
#line 101 "json"
    still_str* t10 = still_json__JsonParser_parse_string_raw(&(*v0));
    still_str* v2 = t10;
#line 102 "json"
    still_bp_check(__FILE__, __LINE__);
#line 102 "json"
    still_str* t11 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t11) != 0)) {
#line 103 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t11) still_release(t11);
    return NULL;
    }
#line 104 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
#line 105 "json"
    still_bp_check(__FILE__, __LINE__);
#line 105 "json"
    still_str* t12 = still_str_new(":", 1);
    if (((!(still_json__JsonParser_expect(&(*v0), t12))))) {
#line 106 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return NULL;
    }
#line 107 "json"
    still_any* t13 = still_json__JsonParser_parse_value(&(*v0));
    still_any* v3 = t13;
#line 108 "json"
    still_bp_check(__FILE__, __LINE__);
#line 108 "json"
    still_str* t14 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t14) != 0)) {
#line 109 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t14) still_release(t14);
    return NULL;
    }
#line 110 "json"
    still_bp_check(__FILE__, __LINE__);
    still_dict_s_a_set(v1, (still_str*)still_retain(v2), (still_any*)still_retain(v3));
#line 111 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
    still_str* t15 = still_json__JsonParser_peek(&(*v0));
    still_str* v4 = t15;
#line 113 "json"
    still_bp_check(__FILE__, __LINE__);
#line 113 "json"
    still_str* t16 = still_str_new(",", 1);
    if ((still_str_cmp(v4, t16) == 0)) {
#line 114 "json"
    still_bp_check(__FILE__, __LINE__);
#line 114 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    if (t16) still_release(t16);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    continue;
    }
#line 116 "json"
    still_bp_check(__FILE__, __LINE__);
#line 116 "json"
    still_str* t17 = still_str_new("}", 1);
    if ((still_str_cmp(v4, t17) == 0)) {
#line 117 "json"
    still_bp_check(__FILE__, __LINE__);
#line 117 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 118 "json"
    still_bp_check(__FILE__, __LINE__);
    still_dict_s_a* t18 = (still_dict_s_a*)still_retain(v1);
    still_any* t19 = still_any_new_p((void*)(t18), ((int64_t)6386532));
    still_release(t18);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t17) still_release(t17);
    return t19;
    }
#line 119 "json"
    still_bp_check(__FILE__, __LINE__);
#line 119 "json"
    still_str* t20 = still_str_new("expected , or closing brace in object", 37);
    (void)(still_json__JsonParser_fail_at(&(*v0), t20));
#line 120 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return NULL;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    }
#line 121 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v1) still_release(v1);
    return NULL;
    if (v1) still_release(v1);
    return NULL;
}

static bool still_json__JsonParser_expect(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1) {
#line 51 "json"
    still_bp_check(__FILE__, __LINE__);
#line 51 "json"
    still_str* t1 = still_json__JsonParser_peek(&(*v0));
    if ((still_str_cmp(t1, v1) != 0)) {
#line 52 "json"
    still_bp_check(__FILE__, __LINE__);
#line 52 "json"
    still_str* t2 = still_str_new("expected ", 9);
    still_str* t3 = (still_str*)still_retain(v1);
    still_str* t4 = still_str_cat(t2, t3);
    bool t5 = still_json__JsonParser_fail_at(&(*v0), t4);
    if (v1) still_release(v1);
    if (t3) still_release(t3);
    if (t2) still_release(t2);
    if (t1) still_release(t1);
    return t5;
    }
#line 53 "json"
    still_bp_check(__FILE__, __LINE__);
#line 53 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 54 "json"
    still_bp_check(__FILE__, __LINE__);
    bool t6 = true;
    if (v1) still_release(v1);
    return t6;
    if (v1) still_release(v1);
    return false;
}

static still_str* still_json__JsonParser_parse_string_raw(still_st_json_u003a__u003a_JsonParser* v0) {
#line 150 "json"
    still_bp_check(__FILE__, __LINE__);
#line 150 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    still_str* t1 = still_str_new("", 0);
    still_str* v1 = t1;
#line 152 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!((true))) {
    break;
    }
#line 153 "json"
    still_bp_check(__FILE__, __LINE__);
    if ((still_json__JsonParser_at_end(&(*v0)))) {
#line 154 "json"
    still_bp_check(__FILE__, __LINE__);
#line 154 "json"
    still_str* t2 = still_str_new("unterminated string", 19);
    (void)(still_json__JsonParser_fail_at(&(*v0), t2));
#line 155 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t3 = (still_str*)still_retain(v1);
    if (v1) still_release(v1);
    return t3;
    }
#line 156 "json"
    still_str* t4 = still_str_index((*v0).text, (*v0).pos, __FILE__, __LINE__);
    still_str* v2 = t4;
#line 157 "json"
    still_bp_check(__FILE__, __LINE__);
#line 157 "json"
    still_str* t5 = still_str_new("\"", 1);
    if ((still_str_cmp(v2, t5) == 0)) {
#line 158 "json"
    still_bp_check(__FILE__, __LINE__);
#line 158 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 159 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t6 = (still_str*)still_retain(v1);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t5) still_release(t5);
    return t6;
    }
#line 160 "json"
    still_bp_check(__FILE__, __LINE__);
#line 160 "json"
    still_str* t7 = still_str_new("\\", 1);
    if ((still_str_cmp(v2, t7) == 0)) {
#line 161 "json"
    still_str* t8 = still_json__JsonParser_peek_at(&(*v0), 1);
    still_str* v3 = t8;
#line 162 "json"
    still_bp_check(__FILE__, __LINE__);
#line 162 "json"
    still_str* t9 = still_str_new("", 0);
    if ((still_str_cmp(v3, t9) == 0)) {
#line 163 "json"
    still_bp_check(__FILE__, __LINE__);
#line 163 "json"
    still_str* t10 = still_str_new("unterminated escape", 19);
    (void)(still_json__JsonParser_fail_at(&(*v0), t10));
#line 164 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t11 = (still_str*)still_retain(v1);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t9) still_release(t9);
    if (t7) still_release(t7);
    return t11;
    }
#line 165 "json"
    still_bp_check(__FILE__, __LINE__);
#line 165 "json"
    still_str* t12 = still_str_new("\"", 1);
    if ((still_str_cmp(v3, t12) == 0)) {
#line 166 "json"
    still_bp_check(__FILE__, __LINE__);
#line 166 "json"
    still_str* t13 = still_str_new("\"", 1);
    still_str_append_inplace(&v1, t13);
    if (t13) still_release(t13);
#line 167 "json"
    still_bp_check(__FILE__, __LINE__);
#line 167 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 168 "json"
    still_bp_check(__FILE__, __LINE__);
#line 168 "json"
    still_str* t14 = still_str_new("\\", 1);
    if ((still_str_cmp(v3, t14) == 0)) {
#line 169 "json"
    still_bp_check(__FILE__, __LINE__);
#line 169 "json"
    still_str* t15 = still_str_new("\\", 1);
    still_str_append_inplace(&v1, t15);
    if (t15) still_release(t15);
#line 170 "json"
    still_bp_check(__FILE__, __LINE__);
#line 170 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 171 "json"
    still_bp_check(__FILE__, __LINE__);
#line 171 "json"
    still_str* t16 = still_str_new("/", 1);
    if ((still_str_cmp(v3, t16) == 0)) {
#line 172 "json"
    still_bp_check(__FILE__, __LINE__);
#line 172 "json"
    still_str* t17 = still_str_new("/", 1);
    still_str_append_inplace(&v1, t17);
    if (t17) still_release(t17);
#line 173 "json"
    still_bp_check(__FILE__, __LINE__);
#line 173 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 174 "json"
    still_bp_check(__FILE__, __LINE__);
#line 174 "json"
    still_str* t18 = still_str_new("b", 1);
    if ((still_str_cmp(v3, t18) == 0)) {
#line 175 "json"
    still_bp_check(__FILE__, __LINE__);
#line 175 "json"
    still_str* t19 = still_chr(8, __FILE__, __LINE__);
    still_str_append_inplace(&v1, t19);
    if (t19) still_release(t19);
#line 176 "json"
    still_bp_check(__FILE__, __LINE__);
#line 176 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 177 "json"
    still_bp_check(__FILE__, __LINE__);
#line 177 "json"
    still_str* t20 = still_str_new("f", 1);
    if ((still_str_cmp(v3, t20) == 0)) {
#line 178 "json"
    still_bp_check(__FILE__, __LINE__);
#line 178 "json"
    still_str* t21 = still_chr(12, __FILE__, __LINE__);
    still_str_append_inplace(&v1, t21);
    if (t21) still_release(t21);
#line 179 "json"
    still_bp_check(__FILE__, __LINE__);
#line 179 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 180 "json"
    still_bp_check(__FILE__, __LINE__);
#line 180 "json"
    still_str* t22 = still_str_new("n", 1);
    if ((still_str_cmp(v3, t22) == 0)) {
#line 181 "json"
    still_bp_check(__FILE__, __LINE__);
#line 181 "json"
    still_str* t23 = still_chr(10, __FILE__, __LINE__);
    still_str_append_inplace(&v1, t23);
    if (t23) still_release(t23);
#line 182 "json"
    still_bp_check(__FILE__, __LINE__);
#line 182 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 183 "json"
    still_bp_check(__FILE__, __LINE__);
#line 183 "json"
    still_str* t24 = still_str_new("r", 1);
    if ((still_str_cmp(v3, t24) == 0)) {
#line 184 "json"
    still_bp_check(__FILE__, __LINE__);
#line 184 "json"
    still_str* t25 = still_chr(13, __FILE__, __LINE__);
    still_str_append_inplace(&v1, t25);
    if (t25) still_release(t25);
#line 185 "json"
    still_bp_check(__FILE__, __LINE__);
#line 185 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 186 "json"
    still_bp_check(__FILE__, __LINE__);
#line 186 "json"
    still_str* t26 = still_str_new("t", 1);
    if ((still_str_cmp(v3, t26) == 0)) {
#line 187 "json"
    still_bp_check(__FILE__, __LINE__);
#line 187 "json"
    still_str* t27 = still_chr(9, __FILE__, __LINE__);
    still_str_append_inplace(&v1, t27);
    if (t27) still_release(t27);
#line 188 "json"
    still_bp_check(__FILE__, __LINE__);
#line 188 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    } else {
#line 189 "json"
    still_bp_check(__FILE__, __LINE__);
#line 189 "json"
    still_str* t28 = still_str_new("u", 1);
    if ((still_str_cmp(v3, t28) == 0)) {
#line 190 "json"
    still_bp_check(__FILE__, __LINE__);
#line 190 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    int64_t v4 = still_json__JsonParser_parse_hex4(&(*v0));
#line 192 "json"
    still_bp_check(__FILE__, __LINE__);
#line 192 "json"
    still_str* t29 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t29) != 0)) {
#line 193 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t30 = (still_str*)still_retain(v1);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t29) still_release(t29);
    if (t28) still_release(t28);
    if (t26) still_release(t26);
    if (t24) still_release(t24);
    if (t22) still_release(t22);
    if (t20) still_release(t20);
    if (t18) still_release(t18);
    if (t16) still_release(t16);
    if (t14) still_release(t14);
    if (t12) still_release(t12);
    return t30;
    }
#line 194 "json"
    still_bp_check(__FILE__, __LINE__);
#line 194 "json"
    bool t31;
    if (((v4 >= 55296))) {
    t31 = (v4 <= 56319);
    } else {
    t31 = (v4 >= 55296);
    }
    if ((t31)) {
#line 196 "json"
    still_bp_check(__FILE__, __LINE__);
#line 196 "json"
    still_str* t32 = still_json__JsonParser_peek(&(*v0));
    still_str* t33 = still_str_new("\\", 1);
    still_str* t34 = still_json__JsonParser_peek_at(&(*v0), 1);
    still_str* t35 = still_str_new("u", 1);
    bool t36;
    if ((still_str_cmp(t32, t33) == 0)) {
    t36 = still_str_cmp(t34, t35) == 0;
    } else {
    t36 = still_str_cmp(t32, t33) == 0;
    }
    if ((t36)) {
#line 197 "json"
    still_bp_check(__FILE__, __LINE__);
#line 197 "json"
    (*v0).pos = still_add((*v0).pos, 2, __FILE__, __LINE__);
    int64_t v5 = still_json__JsonParser_parse_hex4(&(*v0));
#line 199 "json"
    still_bp_check(__FILE__, __LINE__);
#line 199 "json"
    still_str* t37 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t37) != 0)) {
#line 200 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t38 = (still_str*)still_retain(v1);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t37) still_release(t37);
    if (t35) still_release(t35);
    if (t34) still_release(t34);
    if (t33) still_release(t33);
    if (t32) still_release(t32);
    return t38;
    }
#line 201 "json"
    still_bp_check(__FILE__, __LINE__);
#line 201 "json"
    bool t39;
    if (((v5 >= 56320))) {
    t39 = (v5 <= 57343);
    } else {
    t39 = (v5 >= 56320);
    }
    if ((t39)) {
#line 202 "json"
    still_bp_check(__FILE__, __LINE__);
#line 202 "json"
    v4 = still_add(still_add(65536, still_mul(still_sub(v4, 55296, __FILE__, __LINE__), 1024, __FILE__, __LINE__), __FILE__, __LINE__), still_sub(v5, 56320, __FILE__, __LINE__), __FILE__, __LINE__);
    } else {
#line 204 "json"
    still_bp_check(__FILE__, __LINE__);
    v4 = 65533;
    }
    } else {
#line 206 "json"
    still_bp_check(__FILE__, __LINE__);
    v4 = 65533;
    }
    } else {
#line 207 "json"
    still_bp_check(__FILE__, __LINE__);
#line 207 "json"
    bool t40;
    if (((v4 >= 56320))) {
    t40 = (v4 <= 57343);
    } else {
    t40 = (v4 >= 56320);
    }
    if ((t40)) {
#line 208 "json"
    still_bp_check(__FILE__, __LINE__);
    v4 = 65533;
    }
    }
#line 209 "json"
    still_bp_check(__FILE__, __LINE__);
#line 209 "json"
    still_str* t41 = still_json__JsonParser_utf8_encode(&(*v0), v4);
    still_str_append_inplace(&v1, t41);
    } else {
#line 211 "json"
    still_bp_check(__FILE__, __LINE__);
#line 211 "json"
    still_str* t42 = still_str_new("unknown escape sequence", 23);
    (void)(still_json__JsonParser_fail_at(&(*v0), t42));
#line 212 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t43 = (still_str*)still_retain(v1);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t41) still_release(t41);
    return t43;
    }
    }
    }
    }
    }
    }
    }
    }
    }
    if (v3) still_release(v3);
    } else {
#line 214 "json"
    still_bp_check(__FILE__, __LINE__);
#line 214 "json"
    if (((still_ord(v2, __FILE__, __LINE__) < 32))) {
#line 215 "json"
    still_bp_check(__FILE__, __LINE__);
#line 215 "json"
    still_str* t44 = still_str_new("raw control character in string", 31);
    (void)(still_json__JsonParser_fail_at(&(*v0), t44));
#line 216 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t45 = (still_str*)still_retain(v1);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return t45;
    }
#line 217 "json"
    still_bp_check(__FILE__, __LINE__);
#line 217 "json"
    still_str_append_inplace(&v1, v2);
#line 218 "json"
    still_bp_check(__FILE__, __LINE__);
#line 218 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    }
    if (v2) still_release(v2);
    }
#line 219 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t46 = (still_str*)still_retain(v1);
    if (v1) still_release(v1);
    return t46;
    if (v1) still_release(v1);
    return NULL;
}

static still_str* still_json__JsonParser_peek_at(still_st_json_u003a__u003a_JsonParser* v0, int64_t v1) {
#line 33 "json"
    still_bp_check(__FILE__, __LINE__);
#line 33 "json"
    if (((still_add((*v0).pos, v1, __FILE__, __LINE__) >= (*v0).text->len))) {
#line 34 "json"
    still_bp_check(__FILE__, __LINE__);
#line 34 "json"
    still_str* t1 = still_str_new("", 0);
    return t1;
    }
#line 35 "json"
    still_bp_check(__FILE__, __LINE__);
#line 35 "json"
    still_str* t2 = still_str_index((*v0).text, still_add((*v0).pos, v1, __FILE__, __LINE__), __FILE__, __LINE__);
    return t2;
    return NULL;
}

static int64_t still_json__JsonParser_parse_hex4(still_st_json_u003a__u003a_JsonParser* v0) {
#line 222 "json"
    int64_t v1 = 0;
#line 223 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t1 = 0;
    int64_t t2 = 4;
    int64_t t3 = 1;
    for (int64_t v2 = t1; (t3 > 0) ? (v2 < t2) : (v2 > t2); v2 = still_add(v2, t3, __FILE__, __LINE__)) {
    still_str* t4 = still_json__JsonParser_peek(&(*v0));
    still_str* v3 = t4;
    int64_t v4 = still_json__hex_digit((still_str*)still_retain(v3));
#line 226 "json"
    still_bp_check(__FILE__, __LINE__);
#line 226 "json"
    if (((v4 < 0))) {
#line 227 "json"
    still_bp_check(__FILE__, __LINE__);
#line 227 "json"
    still_str* t5 = still_str_new("invalid unicode escape", 22);
    (void)(still_json__JsonParser_fail_at(&(*v0), t5));
#line 228 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t6 = (-(1));
    if (v3) still_release(v3);
    return t6;
    }
#line 229 "json"
    still_bp_check(__FILE__, __LINE__);
#line 229 "json"
    v1 = still_add(still_mul(v1, 16, __FILE__, __LINE__), v4, __FILE__, __LINE__);
#line 230 "json"
    still_bp_check(__FILE__, __LINE__);
#line 230 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    if (v3) still_release(v3);
    }
#line 231 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t7 = v1;
    return t7;
    return 0;
}

static int64_t still_json__hex_digit(still_str* v0) {
#line 297 "json"
    still_str* t1 = still_str_new("0123456789abcdef", 16);
    still_str* v1 = t1;
#line 298 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t2 = 0;
    int64_t t3 = 16;
    int64_t t4 = 1;
    for (int64_t v2 = t2; (t4 > 0) ? (v2 < t3) : (v2 > t3); v2 = still_add(v2, t4, __FILE__, __LINE__)) {
#line 299 "json"
    still_bp_check(__FILE__, __LINE__);
#line 299 "json"
    still_str* t5 = still_str_index(v1, v2, __FILE__, __LINE__);
    if ((still_str_cmp(v0, t5) == 0)) {
#line 300 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t6 = v2;
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t5) still_release(t5);
    return t6;
    }
    }
#line 301 "json"
    still_str* t7 = still_str_new("ABCDEF", 6);
    still_str* v3 = t7;
#line 302 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t8 = 0;
    int64_t t9 = 6;
    int64_t t10 = 1;
    for (int64_t v4 = t8; (t10 > 0) ? (v4 < t9) : (v4 > t9); v4 = still_add(v4, t10, __FILE__, __LINE__)) {
#line 303 "json"
    still_bp_check(__FILE__, __LINE__);
#line 303 "json"
    still_str* t11 = still_str_index(v3, v4, __FILE__, __LINE__);
    if ((still_str_cmp(v0, t11) == 0)) {
#line 304 "json"
    still_bp_check(__FILE__, __LINE__);
#line 304 "json"
    int64_t t12 = still_add(10, v4, __FILE__, __LINE__);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t11) still_release(t11);
    return t12;
    }
    }
#line 305 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t13 = (-(1));
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t13;
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return 0;
}

static still_str* still_json__JsonParser_utf8_encode(still_st_json_u003a__u003a_JsonParser* v0, int64_t v1) {
#line 234 "json"
    still_bp_check(__FILE__, __LINE__);
#line 234 "json"
    if (((v1 < 128))) {
#line 235 "json"
    still_bp_check(__FILE__, __LINE__);
#line 235 "json"
    still_str* t1 = still_chr(v1, __FILE__, __LINE__);
    return t1;
    }
#line 236 "json"
    still_bp_check(__FILE__, __LINE__);
#line 236 "json"
    if (((v1 < 2048))) {
#line 237 "json"
    still_bp_check(__FILE__, __LINE__);
#line 237 "json"
    still_str* t2 = still_chr(still_add(192, (int64_t)(((double)(v1) / (double)(64))), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t3 = still_chr(still_add(128, still_imod(v1, 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t4 = still_str_cat(t2, t3);
    if (t3) still_release(t3);
    if (t2) still_release(t2);
    return t4;
    }
#line 238 "json"
    still_bp_check(__FILE__, __LINE__);
#line 238 "json"
    if (((v1 < 65536))) {
#line 239 "json"
    still_bp_check(__FILE__, __LINE__);
#line 239 "json"
    still_str* t5 = still_chr(still_add(224, (int64_t)(((double)(v1) / (double)(4096))), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t6 = still_chr(still_add(128, still_imod((int64_t)(((double)(v1) / (double)(64))), 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t7 = still_str_cat(t5, t6);
    still_str* t8 = still_chr(still_add(128, still_imod(v1, 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t9 = still_str_cat(t7, t8);
    if (t8) still_release(t8);
    if (t7) still_release(t7);
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    return t9;
    }
#line 240 "json"
    still_bp_check(__FILE__, __LINE__);
#line 240 "json"
    still_str* t10 = still_chr(still_add(240, (int64_t)(((double)(v1) / (double)(262144))), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t11 = still_chr(still_add(128, still_imod((int64_t)(((double)(v1) / (double)(4096))), 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t12 = still_str_cat(t10, t11);
    still_str* t13 = still_chr(still_add(128, still_imod((int64_t)(((double)(v1) / (double)(64))), 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t14 = still_str_cat(t12, t13);
    still_str* t15 = still_chr(still_add(128, still_imod(v1, 64, __FILE__, __LINE__), __FILE__, __LINE__), __FILE__, __LINE__);
    still_str* t16 = still_str_cat(t14, t15);
    if (t15) still_release(t15);
    if (t14) still_release(t14);
    if (t13) still_release(t13);
    if (t12) still_release(t12);
    if (t11) still_release(t11);
    if (t10) still_release(t10);
    return t16;
    return NULL;
}

static still_any* still_json__JsonParser_parse_array(still_st_json_u003a__u003a_JsonParser* v0) {
#line 124 "json"
    still_bp_check(__FILE__, __LINE__);
#line 124 "json"
    still_str* t1 = still_str_new("[", 1);
    if (((!(still_json__JsonParser_expect(&(*v0), t1))))) {
#line 125 "json"
    still_bp_check(__FILE__, __LINE__);
    return NULL;
    }
#line 126 "json"
    still_list_a* t2 = still_list_a_alloc(0);
    still_list_a* v1 = t2;
#line 127 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
#line 128 "json"
    still_bp_check(__FILE__, __LINE__);
#line 128 "json"
    still_str* t3 = still_json__JsonParser_peek(&(*v0));
    still_str* t4 = still_str_new("]", 1);
    if ((still_str_cmp(t3, t4) == 0)) {
#line 129 "json"
    still_bp_check(__FILE__, __LINE__);
#line 129 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 130 "json"
    still_bp_check(__FILE__, __LINE__);
    still_list_a* t5 = (still_list_a*)still_retain(v1);
    still_any* t6 = still_any_new_p((void*)(t5), ((int64_t)24940));
    still_release(t5);
    if (v1) still_release(v1);
    if (t4) still_release(t4);
    if (t3) still_release(t3);
    return t6;
    }
#line 131 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!((true))) {
    break;
    }
#line 132 "json"
    still_any* t7 = still_json__JsonParser_parse_value(&(*v0));
    still_any* v2 = t7;
#line 133 "json"
    still_bp_check(__FILE__, __LINE__);
#line 133 "json"
    still_str* t8 = still_str_new("", 0);
    if ((still_str_cmp((*v0).err, t8) != 0)) {
#line 134 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t8) still_release(t8);
    return NULL;
    }
#line 135 "json"
    still_bp_check(__FILE__, __LINE__);
#line 135 "json"
    (void)(still_list_a_push(v1, (still_any*)still_retain(v2)));
#line 136 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__JsonParser_skip_ws(&(*v0)));
    still_str* t9 = still_json__JsonParser_peek(&(*v0));
    still_str* v3 = t9;
#line 138 "json"
    still_bp_check(__FILE__, __LINE__);
#line 138 "json"
    still_str* t10 = still_str_new(",", 1);
    if ((still_str_cmp(v3, t10) == 0)) {
#line 139 "json"
    still_bp_check(__FILE__, __LINE__);
#line 139 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    if (t10) still_release(t10);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    continue;
    }
#line 141 "json"
    still_bp_check(__FILE__, __LINE__);
#line 141 "json"
    still_str* t11 = still_str_new("]", 1);
    if ((still_str_cmp(v3, t11) == 0)) {
#line 142 "json"
    still_bp_check(__FILE__, __LINE__);
#line 142 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 143 "json"
    still_bp_check(__FILE__, __LINE__);
    still_list_a* t12 = (still_list_a*)still_retain(v1);
    still_any* t13 = still_any_new_p((void*)(t12), ((int64_t)24940));
    still_release(t12);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t11) still_release(t11);
    return t13;
    }
#line 144 "json"
    still_bp_check(__FILE__, __LINE__);
#line 144 "json"
    still_str* t14 = still_str_new("expected , or closing bracket in array", 38);
    (void)(still_json__JsonParser_fail_at(&(*v0), t14));
#line 145 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return NULL;
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    }
#line 146 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v1) still_release(v1);
    return NULL;
    if (v1) still_release(v1);
    return NULL;
}

static still_any* still_json__JsonParser_parse_literal(still_st_json_u003a__u003a_JsonParser* v0, still_str* v1, still_any* v2) {
#line 81 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t1 = 0;
#line 81 "json"
    int64_t t2 = v1->len;
    int64_t t3 = 1;
    for (int64_t v3 = t1; (t3 > 0) ? (v3 < t2) : (v3 > t2); v3 = still_add(v3, t3, __FILE__, __LINE__)) {
#line 82 "json"
    still_bp_check(__FILE__, __LINE__);
#line 82 "json"
    still_str* t4 = still_json__JsonParser_peek(&(*v0));
    still_str* t5 = still_str_index(v1, v3, __FILE__, __LINE__);
    if ((still_str_cmp(t4, t5) != 0)) {
#line 83 "json"
    still_bp_check(__FILE__, __LINE__);
#line 83 "json"
    still_str* t6 = still_str_new("invalid literal, expected ", 26);
    still_str* t7 = (still_str*)still_retain(v1);
    still_str* t8 = still_str_cat(t6, t7);
    (void)(still_json__JsonParser_fail_at(&(*v0), t8));
    if (t7) still_release(t7);
    if (t6) still_release(t6);
#line 84 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (t5) still_release(t5);
    if (t4) still_release(t4);
    return NULL;
    }
#line 85 "json"
    still_bp_check(__FILE__, __LINE__);
#line 85 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    }
#line 86 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t9 = (still_any*)still_retain(v2);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return t9;
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    return NULL;
}

static still_any* still_json__JsonParser_parse_number(still_st_json_u003a__u003a_JsonParser* v0) {
#line 243 "json"
    int64_t v1 = (*v0).pos;
#line 244 "json"
    still_bp_check(__FILE__, __LINE__);
#line 244 "json"
    still_str* t1 = still_json__JsonParser_peek(&(*v0));
    still_str* t2 = still_str_new("-", 1);
    if ((still_str_cmp(t1, t2) == 0)) {
#line 245 "json"
    still_bp_check(__FILE__, __LINE__);
#line 245 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    }
    if (t2) still_release(t2);
    if (t1) still_release(t1);
#line 246 "json"
    still_str* t3 = still_str_new("", 0);
    still_str* v2 = t3;
#line 247 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!(((!(still_json__JsonParser_at_end(&(*v0))))))) {
    break;
    }
#line 248 "json"
    still_str* t4 = still_json__JsonParser_peek(&(*v0));
    still_str* v3 = t4;
#line 249 "json"
    still_bp_check(__FILE__, __LINE__);
#line 249 "json"
    still_str* t5 = still_str_new("0", 1);
    still_str* t6 = still_str_new("9", 1);
    bool t7;
    if ((still_str_cmp(v3, t5) >= 0)) {
    t7 = still_str_cmp(v3, t6) <= 0;
    } else {
    t7 = still_str_cmp(v3, t5) >= 0;
    }
    if ((t7)) {
#line 250 "json"
    still_bp_check(__FILE__, __LINE__);
#line 250 "json"
    still_str_append_inplace(&v2, v3);
#line 251 "json"
    still_bp_check(__FILE__, __LINE__);
#line 251 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    } else {
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    if (v3) still_release(v3);
    break;
    }
    if (v3) still_release(v3);
    }
#line 254 "json"
    still_bp_check(__FILE__, __LINE__);
#line 254 "json"
    if (((v2->len == 0))) {
#line 255 "json"
    still_bp_check(__FILE__, __LINE__);
#line 255 "json"
    still_str* t8 = still_str_new("invalid number", 14);
    (void)(still_json__JsonParser_fail_at(&(*v0), t8));
#line 256 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    return NULL;
    }
#line 257 "json"
    bool v4 = false;
#line 258 "json"
    still_bp_check(__FILE__, __LINE__);
#line 258 "json"
    still_str* t9 = still_json__JsonParser_peek(&(*v0));
    still_str* t10 = still_str_new(".", 1);
    if ((still_str_cmp(t9, t10) == 0)) {
#line 259 "json"
    still_bp_check(__FILE__, __LINE__);
    v4 = true;
#line 260 "json"
    still_bp_check(__FILE__, __LINE__);
#line 260 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    still_str* t11 = still_str_new("", 0);
    still_str* v5 = t11;
#line 262 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!(((!(still_json__JsonParser_at_end(&(*v0))))))) {
    break;
    }
#line 263 "json"
    still_str* t12 = still_json__JsonParser_peek(&(*v0));
    still_str* v6 = t12;
#line 264 "json"
    still_bp_check(__FILE__, __LINE__);
#line 264 "json"
    still_str* t13 = still_str_new("0", 1);
    still_str* t14 = still_str_new("9", 1);
    bool t15;
    if ((still_str_cmp(v6, t13) >= 0)) {
    t15 = still_str_cmp(v6, t14) <= 0;
    } else {
    t15 = still_str_cmp(v6, t13) >= 0;
    }
    if ((t15)) {
#line 265 "json"
    still_bp_check(__FILE__, __LINE__);
#line 265 "json"
    still_str_append_inplace(&v5, v6);
#line 266 "json"
    still_bp_check(__FILE__, __LINE__);
#line 266 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    } else {
    if (t14) still_release(t14);
    if (t13) still_release(t13);
    if (v6) still_release(v6);
    break;
    }
    if (v6) still_release(v6);
    }
#line 269 "json"
    still_bp_check(__FILE__, __LINE__);
#line 269 "json"
    if (((v5->len == 0))) {
#line 270 "json"
    still_bp_check(__FILE__, __LINE__);
#line 270 "json"
    still_str* t16 = still_str_new("digits required after decimal point", 35);
    (void)(still_json__JsonParser_fail_at(&(*v0), t16));
#line 271 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v5) still_release(v5);
    if (v2) still_release(v2);
    if (t10) still_release(t10);
    if (t9) still_release(t9);
    return NULL;
    }
    if (v5) still_release(v5);
    }
#line 272 "json"
    still_bp_check(__FILE__, __LINE__);
#line 272 "json"
    still_str* t17 = still_json__JsonParser_peek(&(*v0));
    still_str* t18 = still_str_new("e", 1);
    still_str* t19 = still_json__JsonParser_peek(&(*v0));
    still_str* t20 = still_str_new("E", 1);
    bool t21;
    if ((still_str_cmp(t17, t18) == 0)) {
    t21 = still_str_cmp(t17, t18) == 0;
    } else {
    t21 = still_str_cmp(t19, t20) == 0;
    }
    if ((t21)) {
#line 273 "json"
    still_bp_check(__FILE__, __LINE__);
    v4 = true;
#line 274 "json"
    still_bp_check(__FILE__, __LINE__);
#line 274 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
#line 275 "json"
    still_bp_check(__FILE__, __LINE__);
#line 275 "json"
    still_str* t22 = still_json__JsonParser_peek(&(*v0));
    still_str* t23 = still_str_new("+", 1);
    still_str* t24 = still_json__JsonParser_peek(&(*v0));
    still_str* t25 = still_str_new("-", 1);
    bool t26;
    if ((still_str_cmp(t22, t23) == 0)) {
    t26 = still_str_cmp(t22, t23) == 0;
    } else {
    t26 = still_str_cmp(t24, t25) == 0;
    }
    if ((t26)) {
#line 276 "json"
    still_bp_check(__FILE__, __LINE__);
#line 276 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    }
    if (t25) still_release(t25);
    if (t24) still_release(t24);
    if (t23) still_release(t23);
    if (t22) still_release(t22);
#line 277 "json"
    still_str* t27 = still_str_new("", 0);
    still_str* v7 = t27;
#line 278 "json"
    still_bp_check(__FILE__, __LINE__);
    for (;;) {
    if (!(((!(still_json__JsonParser_at_end(&(*v0))))))) {
    break;
    }
#line 279 "json"
    still_str* t28 = still_json__JsonParser_peek(&(*v0));
    still_str* v8 = t28;
#line 280 "json"
    still_bp_check(__FILE__, __LINE__);
#line 280 "json"
    still_str* t29 = still_str_new("0", 1);
    still_str* t30 = still_str_new("9", 1);
    bool t31;
    if ((still_str_cmp(v8, t29) >= 0)) {
    t31 = still_str_cmp(v8, t30) <= 0;
    } else {
    t31 = still_str_cmp(v8, t29) >= 0;
    }
    if ((t31)) {
#line 281 "json"
    still_bp_check(__FILE__, __LINE__);
#line 281 "json"
    still_str_append_inplace(&v7, v8);
#line 282 "json"
    still_bp_check(__FILE__, __LINE__);
#line 282 "json"
    (*v0).pos = still_add((*v0).pos, 1, __FILE__, __LINE__);
    } else {
    if (t30) still_release(t30);
    if (t29) still_release(t29);
    if (v8) still_release(v8);
    break;
    }
    if (v8) still_release(v8);
    }
#line 285 "json"
    still_bp_check(__FILE__, __LINE__);
#line 285 "json"
    if (((v7->len == 0))) {
#line 286 "json"
    still_bp_check(__FILE__, __LINE__);
#line 286 "json"
    still_str* t32 = still_str_new("digits required in exponent", 27);
    (void)(still_json__JsonParser_fail_at(&(*v0), t32));
#line 287 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v7) still_release(v7);
    if (v2) still_release(v2);
    if (t20) still_release(t20);
    if (t19) still_release(t19);
    if (t18) still_release(t18);
    if (t17) still_release(t17);
    return NULL;
    }
    if (v7) still_release(v7);
    }
#line 288 "json"
    still_str* t33 = (*v0).text;
    still_str* t34 = still_str_slice(t33, v1, (*v0).pos);
    still_str* v9 = t34;
#line 289 "json"
    still_bp_check(__FILE__, __LINE__);
    if ((v4)) {
#line 290 "json"
    still_bp_check(__FILE__, __LINE__);
#line 290 "json"
    double t35 = still_str_to_float(v9, __FILE__, __LINE__);
    still_any* t36 = still_any_new_f((double)(t35));
    if (v9) still_release(v9);
    if (v2) still_release(v2);
    return t36;
    }
#line 292 "json"
    still_bp_check(__FILE__, __LINE__);
#line 292 "json"
    if (((v2->len > 18))) {
#line 293 "json"
    still_bp_check(__FILE__, __LINE__);
#line 293 "json"
    double t37 = still_str_to_float(v9, __FILE__, __LINE__);
    still_any* t38 = still_any_new_f((double)(t37));
    if (v9) still_release(v9);
    if (v2) still_release(v2);
    return t38;
    }
#line 294 "json"
    still_bp_check(__FILE__, __LINE__);
#line 294 "json"
    int64_t t39 = still_str_to_int(v9, __FILE__, __LINE__);
    still_any* t40 = still_any_new_i((int64_t)(t39));
    if (v9) still_release(v9);
    if (v2) still_release(v2);
    return t40;
    if (v9) still_release(v9);
    if (v2) still_release(v2);
    return NULL;
}

static still_str* still_json__get_str(still_any* v0, still_str* v1, still_str* v2) {
#line 336 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 337 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 338 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t2 = (still_str*)still_retain(v2);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 339 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* v4 = t4;
#line 340 "json"
    still_bp_check(__FILE__, __LINE__);
#line 340 "json"
    still_str* t5 = still_str_new("str", 3);
    if ((still_str_cmp(v4, t5) == 0)) {
#line 341 "json"
    still_bp_check(__FILE__, __LINE__);
#line 341 "json"
    still_str* t6 = still_any_tostr(v3);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t5) still_release(t5);
    return t6;
    }
#line 342 "json"
    still_bp_check(__FILE__, __LINE__);
#line 342 "json"
    still_str* t7 = still_str_new("int", 3);
    still_str* t8 = still_str_new("float", 5);
    bool t9;
    if ((still_str_cmp(v4, t7) == 0)) {
    t9 = still_str_cmp(v4, t7) == 0;
    } else {
    t9 = still_str_cmp(v4, t8) == 0;
    }
    still_str* t10 = still_str_new("bool", 4);
    bool t11;
    if ((t9)) {
    t11 = t9;
    } else {
    t11 = still_str_cmp(v4, t10) == 0;
    }
    if ((t11)) {
#line 343 "json"
    still_bp_check(__FILE__, __LINE__);
#line 343 "json"
    still_str* t12 = still_any_tostr(v3);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t10) still_release(t10);
    if (t8) still_release(t8);
    if (t7) still_release(t7);
    return t12;
    }
#line 344 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t13 = (still_str*)still_retain(v2);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t13;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
}

static still_any* still_json__get_any(still_any* v0, still_str* v1, still_any* v2) {
#line 327 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v0 == NULL || ((v0)->tag & 0xFF) == (int)'n'))) {
#line 328 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t1 = (still_any*)still_retain(v2);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t1;
    }
#line 329 "json"
    still_bp_check(__FILE__, __LINE__);
#line 329 "json"
    const char* t2 = still_any_typename(v0);
    still_str* t3 = still_str_new(t2, strlen(t2));
    still_str* t4 = still_str_new("dict", 4);
    if ((still_str_cmp(t3, t4) != 0)) {
#line 330 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t5 = (still_any*)still_retain(v2);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t4) still_release(t4);
    if (t3) still_release(t3);
    return t5;
    }
#line 331 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t6 = still_any_new_s(v1);
    if (((!(still_any_in(t6, v0))))) {
#line 332 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t7 = (still_any*)still_retain(v2);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t6) still_release(t6);
    return t7;
    }
#line 333 "json"
    still_bp_check(__FILE__, __LINE__);
#line 333 "json"
    still_any* t8 = still_any_index_s(v0, v1);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t8;
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
}

static int64_t still_json__get_int(still_any* v0, still_str* v1, int64_t v2) {
#line 347 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 348 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 349 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t2 = v2;
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 350 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* v4 = t4;
#line 351 "json"
    still_bp_check(__FILE__, __LINE__);
#line 351 "json"
    still_str* t5 = still_str_new("int", 3);
    if ((still_str_cmp(v4, t5) == 0)) {
#line 352 "json"
    still_bp_check(__FILE__, __LINE__);
#line 352 "json"
    int64_t t6 = still_any_to_int(v3, __FILE__, __LINE__);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t5) still_release(t5);
    return t6;
    }
#line 353 "json"
    still_bp_check(__FILE__, __LINE__);
#line 353 "json"
    still_str* t7 = still_str_new("float", 5);
    if ((still_str_cmp(v4, t7) == 0)) {
#line 354 "json"
    still_bp_check(__FILE__, __LINE__);
#line 354 "json"
    int64_t t8 = still_any_to_int(v3, __FILE__, __LINE__);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t7) still_release(t7);
    return t8;
    }
#line 355 "json"
    still_bp_check(__FILE__, __LINE__);
#line 355 "json"
    still_str* t9 = still_str_new("str", 3);
    if ((still_str_cmp(v4, t9) == 0)) {
#line 356 "json"
    still_bp_check(__FILE__, __LINE__);
#line 356 "json"
    int64_t t10 = still_any_to_int(v3, __FILE__, __LINE__);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t9) still_release(t9);
    return t10;
    }
#line 357 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t11 = v2;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t11;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return 0;
}

static double still_json__get_float(still_any* v0, still_str* v1, double v2) {
#line 360 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 361 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 362 "json"
    still_bp_check(__FILE__, __LINE__);
    double t2 = v2;
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 363 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* v4 = t4;
#line 364 "json"
    still_bp_check(__FILE__, __LINE__);
#line 364 "json"
    still_str* t5 = still_str_new("float", 5);
    still_str* t6 = still_str_new("int", 3);
    bool t7;
    if ((still_str_cmp(v4, t5) == 0)) {
    t7 = still_str_cmp(v4, t5) == 0;
    } else {
    t7 = still_str_cmp(v4, t6) == 0;
    }
    still_str* t8 = still_str_new("str", 3);
    bool t9;
    if ((t7)) {
    t9 = t7;
    } else {
    t9 = still_str_cmp(v4, t8) == 0;
    }
    if ((t9)) {
#line 365 "json"
    still_bp_check(__FILE__, __LINE__);
#line 365 "json"
    double t10 = still_any_to_float(v3, __FILE__, __LINE__);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t8) still_release(t8);
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    return t10;
    }
#line 366 "json"
    still_bp_check(__FILE__, __LINE__);
    double t11 = v2;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t11;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return 0.0;
}

static bool still_json__get_bool(still_any* v0, still_str* v1, bool v2) {
#line 369 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 370 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 371 "json"
    still_bp_check(__FILE__, __LINE__);
    bool t2 = v2;
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 372 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* v4 = t4;
#line 373 "json"
    still_bp_check(__FILE__, __LINE__);
#line 373 "json"
    still_str* t5 = still_str_new("bool", 4);
    if ((still_str_cmp(v4, t5) == 0)) {
#line 374 "json"
    still_bp_check(__FILE__, __LINE__);
#line 374 "json"
    still_any* t6 = still_any_new_b((int64_t)(true));
    bool t7 = still_any_cmp(0, v3, t6);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    return t7;
    }
#line 375 "json"
    still_bp_check(__FILE__, __LINE__);
#line 375 "json"
    still_str* t8 = still_str_new("int", 3);
    if ((still_str_cmp(v4, t8) == 0)) {
#line 376 "json"
    still_bp_check(__FILE__, __LINE__);
#line 376 "json"
    still_any* t9 = still_any_new_i((int64_t)(0));
    bool t10 = still_any_cmp(1, v3, t9);
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t9) still_release(t9);
    if (t8) still_release(t8);
    return t10;
    }
#line 377 "json"
    still_bp_check(__FILE__, __LINE__);
    bool t11 = v2;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t11;
    if (v4) still_release(v4);
    if (v3) still_release(v3);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return false;
}

static still_any* still_json__get_list(still_any* v0, still_str* v1, still_any* v2) {
#line 380 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 381 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 382 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t2 = (still_any*)still_retain(v2);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 383 "json"
    still_bp_check(__FILE__, __LINE__);
#line 383 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* t5 = still_str_new("list", 4);
    if ((still_str_cmp(t4, t5) == 0)) {
#line 384 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t6 = (still_any*)still_retain(v3);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t5) still_release(t5);
    if (t4) still_release(t4);
    return t6;
    }
#line 385 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t7 = (still_any*)still_retain(v2);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t7;
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
}

static still_any* still_json__get_dict(still_any* v0, still_str* v1, still_any* v2) {
#line 388 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v0), (still_str*)still_retain(v1), NULL);
    still_any* v3 = t1;
#line 389 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v3 == NULL || ((v3)->tag & 0xFF) == (int)'n'))) {
#line 390 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t2 = (still_any*)still_retain(v2);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    }
#line 391 "json"
    still_bp_check(__FILE__, __LINE__);
#line 391 "json"
    const char* t3 = still_any_typename(v3);
    still_str* t4 = still_str_new(t3, strlen(t3));
    still_str* t5 = still_str_new("dict", 4);
    if ((still_str_cmp(t4, t5) == 0)) {
#line 392 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t6 = (still_any*)still_retain(v3);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    if (t5) still_release(t5);
    if (t4) still_release(t4);
    return t6;
    }
#line 393 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t7 = (still_any*)still_retain(v2);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t7;
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
}

static still_str* still_json__stringify(still_any* v0) {
#line 413 "json"
    still_list_s* t2 = still_list_s_alloc(0);
    still_st_json_u003a__u003a_StringBuilder t1 = { .parts = t2 };
    still_st_json_u003a__u003a_StringBuilder v1 = t1;
#line 414 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__write_value(still_st_json_u003a__u003a_StringBuilder_clone(&v1), (still_any*)still_retain(v0)));
#line 415 "json"
    still_bp_check(__FILE__, __LINE__);
#line 415 "json"
    still_str* t3 = still_json__StringBuilder_to_string(&v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v1);
    if (v0) still_release(v0);
    return t3;
    still_st_json_u003a__u003a_StringBuilder_drop(&v1);
    if (v0) still_release(v0);
    return NULL;
}

static void still_json__write_value(still_st_json_u003a__u003a_StringBuilder v0, still_any* v1) {
#line 427 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v1 == NULL || ((v1)->tag & 0xFF) == (int)'n'))) {
#line 428 "json"
    still_bp_check(__FILE__, __LINE__);
#line 428 "json"
    still_str* t1 = still_str_new("null", 4);
    (void)(still_json__StringBuilder_append(&v0, t1));
#line 429 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    NULL;
    return;
    }
#line 430 "json"
    const char* t2 = still_any_typename(v1);
    still_str* t3 = still_str_new(t2, strlen(t2));
    still_str* v2 = t3;
#line 431 "json"
    still_bp_check(__FILE__, __LINE__);
#line 431 "json"
    still_str* t4 = still_str_new("str", 3);
    if ((still_str_cmp(v2, t4) == 0)) {
#line 432 "json"
    still_bp_check(__FILE__, __LINE__);
#line 432 "json"
    still_str* t5 = still_any_tostr(v1);
    (void)(still_json__write_string(still_st_json_u003a__u003a_StringBuilder_clone(&v0), t5));
#line 433 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t4) still_release(t4);
    NULL;
    return;
    }
#line 434 "json"
    still_bp_check(__FILE__, __LINE__);
#line 434 "json"
    still_str* t6 = still_str_new("int", 3);
    if ((still_str_cmp(v2, t6) == 0)) {
#line 435 "json"
    still_bp_check(__FILE__, __LINE__);
#line 435 "json"
    still_str* t7 = still_str_from_int(still_any_to_int(v1, __FILE__, __LINE__));
    (void)(still_json__StringBuilder_append(&v0, t7));
#line 436 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t6) still_release(t6);
    NULL;
    return;
    }
#line 437 "json"
    still_bp_check(__FILE__, __LINE__);
#line 437 "json"
    still_str* t8 = still_str_new("float", 5);
    if ((still_str_cmp(v2, t8) == 0)) {
#line 438 "json"
    double v3 = still_any_to_float(v1, __FILE__, __LINE__);
#line 439 "json"
    still_bp_check(__FILE__, __LINE__);
#line 439 "json"
    bool t9;
    if (((v3 == (double)((int64_t)(v3))))) {
    t9 = (v3 < 1000000000000000);
    } else {
    t9 = (v3 == (double)((int64_t)(v3)));
    }
    bool t10;
    if ((t9)) {
    t10 = (v3 > (-(1000000000000000)));
    } else {
    t10 = t9;
    }
    if ((t10)) {
#line 440 "json"
    still_bp_check(__FILE__, __LINE__);
#line 440 "json"
    still_str* t11 = still_str_from_int((int64_t)(v3));
    still_str* t12 = still_str_new(".0", 2);
    still_str* t13 = still_str_cat(t11, t12);
    (void)(still_json__StringBuilder_append(&v0, t13));
    if (t12) still_release(t12);
    if (t11) still_release(t11);
    } else {
#line 442 "json"
    still_bp_check(__FILE__, __LINE__);
#line 442 "json"
    still_str* t14 = still_str_from_float(v3);
    (void)(still_json__StringBuilder_append(&v0, t14));
    }
#line 443 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t8) still_release(t8);
    NULL;
    return;
    }
#line 444 "json"
    still_bp_check(__FILE__, __LINE__);
#line 444 "json"
    still_str* t15 = still_str_new("bool", 4);
    if ((still_str_cmp(v2, t15) == 0)) {
#line 445 "json"
    still_bp_check(__FILE__, __LINE__);
#line 445 "json"
    still_any* t16 = still_any_new_b((int64_t)(true));
    if ((still_any_cmp(0, v1, t16))) {
#line 446 "json"
    still_bp_check(__FILE__, __LINE__);
#line 446 "json"
    still_str* t17 = still_str_new("true", 4);
    (void)(still_json__StringBuilder_append(&v0, t17));
    } else {
#line 448 "json"
    still_bp_check(__FILE__, __LINE__);
#line 448 "json"
    still_str* t18 = still_str_new("false", 5);
    (void)(still_json__StringBuilder_append(&v0, t18));
    }
    if (t16) still_release(t16);
#line 449 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t15) still_release(t15);
    NULL;
    return;
    }
#line 450 "json"
    still_bp_check(__FILE__, __LINE__);
#line 450 "json"
    still_str* t19 = still_str_new("list", 4);
    if ((still_str_cmp(v2, t19) == 0)) {
#line 451 "json"
    still_bp_check(__FILE__, __LINE__);
#line 451 "json"
    still_str* t20 = still_str_new("[", 1);
    (void)(still_json__StringBuilder_append(&v0, t20));
#line 452 "json"
    int64_t v4 = still_any_len(v1);
#line 453 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t21 = 0;
    int64_t t22 = 1;
    for (int64_t v5 = t21; (t22 > 0) ? (v5 < v4) : (v5 > v4); v5 = still_add(v5, t22, __FILE__, __LINE__)) {
#line 454 "json"
    still_bp_check(__FILE__, __LINE__);
#line 454 "json"
    if (((v5 > 0))) {
#line 455 "json"
    still_bp_check(__FILE__, __LINE__);
#line 455 "json"
    still_str* t23 = still_str_new(",", 1);
    (void)(still_json__StringBuilder_append(&v0, t23));
    }
#line 456 "json"
    still_bp_check(__FILE__, __LINE__);
#line 456 "json"
    still_any* t24 = still_any_index_i(v1, v5);
    (void)(still_json__write_value(still_st_json_u003a__u003a_StringBuilder_clone(&v0), t24));
    }
#line 457 "json"
    still_bp_check(__FILE__, __LINE__);
#line 457 "json"
    still_str* t25 = still_str_new("]", 1);
    (void)(still_json__StringBuilder_append(&v0, t25));
#line 458 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t19) still_release(t19);
    NULL;
    return;
    }
#line 459 "json"
    still_bp_check(__FILE__, __LINE__);
#line 459 "json"
    still_str* t26 = still_str_new("dict", 4);
    if ((still_str_cmp(v2, t26) == 0)) {
#line 460 "json"
    still_bp_check(__FILE__, __LINE__);
#line 460 "json"
    still_str* t27 = still_str_new("{", 1);
    (void)(still_json__StringBuilder_append(&v0, t27));
#line 461 "json"
    int64_t v6 = still_any_len(v1);
#line 462 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t28 = 0;
    int64_t t29 = 1;
    for (int64_t v7 = t28; (t29 > 0) ? (v7 < v6) : (v7 > v6); v7 = still_add(v7, t29, __FILE__, __LINE__)) {
#line 463 "json"
    still_bp_check(__FILE__, __LINE__);
#line 463 "json"
    if (((v7 > 0))) {
#line 464 "json"
    still_bp_check(__FILE__, __LINE__);
#line 464 "json"
    still_str* t30 = still_str_new(",", 1);
    (void)(still_json__StringBuilder_append(&v0, t30));
    }
#line 465 "json"
    still_str* t31 = still_dict_key_at(v1, v7);
    still_str* v8 = t31;
#line 466 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__write_string(still_st_json_u003a__u003a_StringBuilder_clone(&v0), (still_str*)still_retain(v8)));
#line 467 "json"
    still_bp_check(__FILE__, __LINE__);
#line 467 "json"
    still_str* t32 = still_str_new(":", 1);
    (void)(still_json__StringBuilder_append(&v0, t32));
#line 468 "json"
    still_bp_check(__FILE__, __LINE__);
#line 468 "json"
    still_any* t33 = still_any_index_s(v1, v8);
    (void)(still_json__write_value(still_st_json_u003a__u003a_StringBuilder_clone(&v0), t33));
    if (v8) still_release(v8);
    }
#line 469 "json"
    still_bp_check(__FILE__, __LINE__);
#line 469 "json"
    still_str* t34 = still_str_new("}", 1);
    (void)(still_json__StringBuilder_append(&v0, t34));
#line 470 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    if (t26) still_release(t26);
    NULL;
    return;
    }
#line 471 "json"
    still_bp_check(__FILE__, __LINE__);
#line 471 "json"
    still_str* t35 = still_str_new("null", 4);
    (void)(still_json__StringBuilder_append(&v0, t35));
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    return;
}

static void still_json__StringBuilder_append(still_st_json_u003a__u003a_StringBuilder* v0, still_str* v1) {
#line 421 "json"
    still_bp_check(__FILE__, __LINE__);
#line 421 "json"
    (void)(still_list_s_push((*v0).parts, (still_str*)still_retain(v1)));
    if (v1) still_release(v1);
    return;
}

static void still_json__write_string(still_st_json_u003a__u003a_StringBuilder v0, still_str* v1) {
#line 474 "json"
    still_bp_check(__FILE__, __LINE__);
#line 474 "json"
    still_str* t1 = still_str_new("\"", 1);
    (void)(still_json__StringBuilder_append(&v0, t1));
#line 475 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t2 = 0;
#line 475 "json"
    int64_t t3 = v1->len;
    int64_t t4 = 1;
    for (int64_t v2 = t2; (t4 > 0) ? (v2 < t3) : (v2 > t3); v2 = still_add(v2, t4, __FILE__, __LINE__)) {
#line 476 "json"
    still_str* t5 = still_str_index(v1, v2, __FILE__, __LINE__);
    still_str* v3 = t5;
    int64_t v4 = still_ord(v3, __FILE__, __LINE__);
#line 478 "json"
    still_bp_check(__FILE__, __LINE__);
#line 478 "json"
    still_str* t6 = still_str_new("\"", 1);
    if ((still_str_cmp(v3, t6) == 0)) {
#line 479 "json"
    still_bp_check(__FILE__, __LINE__);
#line 479 "json"
    still_str* t7 = still_str_new("\\\"", 2);
    (void)(still_json__StringBuilder_append(&v0, t7));
    } else {
#line 480 "json"
    still_bp_check(__FILE__, __LINE__);
#line 480 "json"
    still_str* t8 = still_str_new("\\", 1);
    if ((still_str_cmp(v3, t8) == 0)) {
#line 481 "json"
    still_bp_check(__FILE__, __LINE__);
#line 481 "json"
    still_str* t9 = still_str_new("\\\\", 2);
    (void)(still_json__StringBuilder_append(&v0, t9));
    } else {
#line 482 "json"
    still_bp_check(__FILE__, __LINE__);
#line 482 "json"
    still_str* t10 = still_str_new("\n", 1);
    if ((still_str_cmp(v3, t10) == 0)) {
#line 483 "json"
    still_bp_check(__FILE__, __LINE__);
#line 483 "json"
    still_str* t11 = still_str_new("\\n", 2);
    (void)(still_json__StringBuilder_append(&v0, t11));
    } else {
#line 484 "json"
    still_bp_check(__FILE__, __LINE__);
#line 484 "json"
    still_str* t12 = still_str_new("\r", 1);
    if ((still_str_cmp(v3, t12) == 0)) {
#line 485 "json"
    still_bp_check(__FILE__, __LINE__);
#line 485 "json"
    still_str* t13 = still_str_new("\\r", 2);
    (void)(still_json__StringBuilder_append(&v0, t13));
    } else {
#line 486 "json"
    still_bp_check(__FILE__, __LINE__);
#line 486 "json"
    still_str* t14 = still_str_new("\t", 1);
    if ((still_str_cmp(v3, t14) == 0)) {
#line 487 "json"
    still_bp_check(__FILE__, __LINE__);
#line 487 "json"
    still_str* t15 = still_str_new("\\t", 2);
    (void)(still_json__StringBuilder_append(&v0, t15));
    } else {
#line 488 "json"
    still_bp_check(__FILE__, __LINE__);
#line 488 "json"
    if (((v4 == 8))) {
#line 489 "json"
    still_bp_check(__FILE__, __LINE__);
#line 489 "json"
    still_str* t16 = still_str_new("\\b", 2);
    (void)(still_json__StringBuilder_append(&v0, t16));
    } else {
#line 490 "json"
    still_bp_check(__FILE__, __LINE__);
#line 490 "json"
    if (((v4 == 12))) {
#line 491 "json"
    still_bp_check(__FILE__, __LINE__);
#line 491 "json"
    still_str* t17 = still_str_new("\\f", 2);
    (void)(still_json__StringBuilder_append(&v0, t17));
    } else {
#line 492 "json"
    still_bp_check(__FILE__, __LINE__);
#line 492 "json"
    if (((v4 < 32))) {
#line 493 "json"
    still_bp_check(__FILE__, __LINE__);
#line 493 "json"
    still_str* t18 = still_str_new("\\u", 2);
    still_str* t19 = still_json__hex4(v4);
    still_str* t20 = t19;
    still_str* t21 = still_str_cat(t18, t20);
    (void)(still_json__StringBuilder_append(&v0, t21));
    if (t20) still_release(t20);
    if (t18) still_release(t18);
    } else {
#line 495 "json"
    still_bp_check(__FILE__, __LINE__);
    (void)(still_json__StringBuilder_append(&v0, (still_str*)still_retain(v3)));
    }
    }
    }
    }
    if (t14) still_release(t14);
    }
    if (t12) still_release(t12);
    }
    if (t10) still_release(t10);
    }
    if (t8) still_release(t8);
    }
    if (t6) still_release(t6);
    if (v3) still_release(v3);
    }
#line 496 "json"
    still_bp_check(__FILE__, __LINE__);
#line 496 "json"
    still_str* t22 = still_str_new("\"", 1);
    (void)(still_json__StringBuilder_append(&v0, t22));
    if (v1) still_release(v1);
    still_st_json_u003a__u003a_StringBuilder_drop(&v0);
    return;
}

static still_str* still_json__hex4(int64_t v0) {
#line 499 "json"
    still_str* t1 = still_str_new("0123456789abcdef", 16);
    still_str* v1 = t1;
    int64_t v2 = still_imod((int64_t)(((double)(v0) / (double)(4096))), 16, __FILE__, __LINE__);
    int64_t v3 = still_imod((int64_t)(((double)(v0) / (double)(256))), 16, __FILE__, __LINE__);
    int64_t v4 = still_imod((int64_t)(((double)(v0) / (double)(16))), 16, __FILE__, __LINE__);
    int64_t v5 = still_imod(v0, 16, __FILE__, __LINE__);
#line 504 "json"
    still_bp_check(__FILE__, __LINE__);
#line 504 "json"
    still_str* t2 = still_str_index(v1, v2, __FILE__, __LINE__);
    still_str* t3 = still_str_index(v1, v3, __FILE__, __LINE__);
    still_str* t4 = still_str_cat(t2, t3);
    still_str* t5 = still_str_index(v1, v4, __FILE__, __LINE__);
    still_str* t6 = still_str_cat(t4, t5);
    still_str* t7 = still_str_index(v1, v5, __FILE__, __LINE__);
    still_str* t8 = still_str_cat(t6, t7);
    if (v1) still_release(v1);
    if (t7) still_release(t7);
    if (t6) still_release(t6);
    if (t5) still_release(t5);
    if (t4) still_release(t4);
    if (t3) still_release(t3);
    if (t2) still_release(t2);
    return t8;
    if (v1) still_release(v1);
    return NULL;
}

static still_str* still_json__StringBuilder_to_string(still_st_json_u003a__u003a_StringBuilder* v0) {
#line 424 "json"
    still_bp_check(__FILE__, __LINE__);
#line 424 "json"
    still_str* t1 = still_str_new("", 0);
    still_str* t2 = still_str_join(t1, (*v0).parts);
    if (t1) still_release(t1);
    return t2;
    return NULL;
}

static bool still_json__valid(still_str* v0) {
#line 319 "json"
    still_st_json_u003a__u003a_JsonResult t1 = still_json__parse((still_str*)still_retain(v0));
    still_st_json_u003a__u003a_JsonResult v1 = t1;
#line 320 "json"
    still_bp_check(__FILE__, __LINE__);
    bool t2 = v1.ok;
    still_st_json_u003a__u003a_JsonResult_drop(&v1);
    if (v0) still_release(v0);
    return t2;
    still_st_json_u003a__u003a_JsonResult_drop(&v1);
    if (v0) still_release(v0);
    return false;
}

static still_str* still_json__stringify_pretty(still_any* v0, int64_t v1) {
#line 508 "json"
    still_str* t1 = still_str_new("", 0);
    still_str* v2 = t1;
#line 509 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t2 = 0;
    int64_t t3 = 1;
    for (int64_t v3 = t2; (t3 > 0) ? (v3 < v1) : (v3 > v1); v3 = still_add(v3, t3, __FILE__, __LINE__)) {
#line 510 "json"
    still_bp_check(__FILE__, __LINE__);
#line 510 "json"
    still_str* t4 = still_str_new("  ", 2);
    still_str_append_inplace(&v2, t4);
    if (t4) still_release(t4);
    }
#line 511 "json"
    still_str* t5 = still_str_new("", 0);
    still_str* v4 = t5;
#line 512 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t6 = 0;
#line 512 "json"
    int64_t t7 = still_add(v1, 1, __FILE__, __LINE__);
    int64_t t8 = 1;
    for (int64_t v5 = t6; (t8 > 0) ? (v5 < t7) : (v5 > t7); v5 = still_add(v5, t8, __FILE__, __LINE__)) {
#line 513 "json"
    still_bp_check(__FILE__, __LINE__);
#line 513 "json"
    still_str* t9 = still_str_new("  ", 2);
    still_str_append_inplace(&v4, t9);
    if (t9) still_release(t9);
    }
#line 514 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v0 == NULL || ((v0)->tag & 0xFF) == (int)'n'))) {
#line 515 "json"
    still_bp_check(__FILE__, __LINE__);
#line 515 "json"
    still_str* t10 = still_str_new("null", 4);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    return t10;
    }
#line 516 "json"
    const char* t11 = still_any_typename(v0);
    still_str* t12 = still_str_new(t11, strlen(t11));
    still_str* v6 = t12;
#line 517 "json"
    still_bp_check(__FILE__, __LINE__);
#line 517 "json"
    still_str* t13 = still_str_new("list", 4);
    if ((still_str_cmp(v6, t13) == 0)) {
#line 518 "json"
    int64_t v7 = still_any_len(v0);
#line 519 "json"
    still_bp_check(__FILE__, __LINE__);
#line 519 "json"
    if (((v7 == 0))) {
#line 520 "json"
    still_bp_check(__FILE__, __LINE__);
#line 520 "json"
    still_str* t14 = still_str_new("[]", 2);
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    if (t13) still_release(t13);
    return t14;
    }
#line 521 "json"
    still_str* t15 = still_str_new("[\n", 2);
    still_str* v8 = t15;
#line 522 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t16 = 0;
    int64_t t17 = 1;
    for (int64_t v9 = t16; (t17 > 0) ? (v9 < v7) : (v9 > v7); v9 = still_add(v9, t17, __FILE__, __LINE__)) {
#line 523 "json"
    still_bp_check(__FILE__, __LINE__);
#line 523 "json"
    if (((v9 > 0))) {
#line 524 "json"
    still_bp_check(__FILE__, __LINE__);
#line 524 "json"
    still_str* t18 = still_str_new(",\n", 2);
    still_str_append_inplace(&v8, t18);
    if (t18) still_release(t18);
    }
#line 525 "json"
    still_bp_check(__FILE__, __LINE__);
#line 525 "json"
    still_any* t19 = still_any_index_i(v0, v9);
    still_str* t20 = still_json__stringify_pretty(t19, still_add(v1, 1, __FILE__, __LINE__));
    still_str* t21 = still_str_cat(v4, t20);
    still_str_append_inplace(&v8, t21);
    if (t21) still_release(t21);
    if (t20) still_release(t20);
    }
#line 526 "json"
    still_bp_check(__FILE__, __LINE__);
#line 526 "json"
    still_str* t22 = still_str_new("\n", 1);
    still_str* t23 = still_str_cat(t22, v2);
    still_str* t24 = still_str_new("]", 1);
    still_str* t25 = still_str_cat(t23, t24);
    still_str_append_inplace(&v8, t25);
    if (t25) still_release(t25);
    if (t24) still_release(t24);
    if (t23) still_release(t23);
#line 527 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t26 = (still_str*)still_retain(v8);
    if (v8) still_release(v8);
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    if (t22) still_release(t22);
    return t26;
    if (v8) still_release(v8);
    }
#line 528 "json"
    still_bp_check(__FILE__, __LINE__);
#line 528 "json"
    still_str* t27 = still_str_new("dict", 4);
    if ((still_str_cmp(v6, t27) == 0)) {
#line 529 "json"
    int64_t v10 = still_any_len(v0);
#line 530 "json"
    still_bp_check(__FILE__, __LINE__);
#line 530 "json"
    if (((v10 == 0))) {
#line 531 "json"
    still_bp_check(__FILE__, __LINE__);
#line 531 "json"
    still_str* t28 = still_str_new("{}", 2);
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    if (t27) still_release(t27);
    return t28;
    }
#line 532 "json"
    still_str* t29 = still_str_new("{\n", 2);
    still_str* v11 = t29;
#line 533 "json"
    still_bp_check(__FILE__, __LINE__);
    int64_t t30 = 0;
    int64_t t31 = 1;
    for (int64_t v12 = t30; (t31 > 0) ? (v12 < v10) : (v12 > v10); v12 = still_add(v12, t31, __FILE__, __LINE__)) {
    still_str* t32 = still_dict_key_at(v0, v12);
    still_str* v13 = t32;
#line 535 "json"
    still_bp_check(__FILE__, __LINE__);
#line 535 "json"
    if (((v12 > 0))) {
#line 536 "json"
    still_bp_check(__FILE__, __LINE__);
#line 536 "json"
    still_str* t33 = still_str_new(",\n", 2);
    still_str_append_inplace(&v11, t33);
    if (t33) still_release(t33);
    }
#line 537 "json"
    still_bp_check(__FILE__, __LINE__);
#line 537 "json"
    still_str* t34 = still_json__stringify(still_any_new_s(v13));
    still_str* t35 = still_str_cat(v4, t34);
    still_str* t36 = still_str_new(": ", 2);
    still_str* t37 = still_str_cat(t35, t36);
    still_any* t38 = still_any_index_s(v0, v13);
    still_str* t39 = still_json__stringify_pretty(t38, still_add(v1, 1, __FILE__, __LINE__));
    still_str* t40 = still_str_cat(t37, t39);
    still_str_append_inplace(&v11, t40);
    if (t40) still_release(t40);
    if (t39) still_release(t39);
    if (t37) still_release(t37);
    if (t36) still_release(t36);
    if (t35) still_release(t35);
    if (t34) still_release(t34);
    if (v13) still_release(v13);
    }
#line 538 "json"
    still_bp_check(__FILE__, __LINE__);
#line 538 "json"
    still_str* t41 = still_str_new("\n", 1);
    still_str* t42 = still_str_cat(t41, v2);
    still_str* t43 = still_str_new("}", 1);
    still_str* t44 = still_str_cat(t42, t43);
    still_str_append_inplace(&v11, t44);
    if (t44) still_release(t44);
    if (t43) still_release(t43);
    if (t42) still_release(t42);
#line 539 "json"
    still_bp_check(__FILE__, __LINE__);
    still_str* t45 = (still_str*)still_retain(v11);
    if (v11) still_release(v11);
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    if (t41) still_release(t41);
    return t45;
    if (v11) still_release(v11);
    }
#line 540 "json"
    still_bp_check(__FILE__, __LINE__);
#line 540 "json"
    still_str* t46 = still_json__stringify((still_any*)still_retain(v0));
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    return t46;
    if (v6) still_release(v6);
    if (v4) still_release(v4);
    if (v2) still_release(v2);
    if (v0) still_release(v0);
    return NULL;
}

static still_any* still_json__get_at(still_any* v0, still_list_s* v1) {
#line 397 "json"
    still_any* v2 = (still_any*)still_retain(v0);
#line 398 "json"
    still_bp_check(__FILE__, __LINE__);
    for (int64_t __i = 0; __i < v1->len; __i++) {
    still_str* v3 = (still_str*)still_retain(v1->data[__i]);
#line 399 "json"
    still_bp_check(__FILE__, __LINE__);
#line 399 "json"
    still_any* t1 = still_json__get_any((still_any*)still_retain(v2), (still_str*)still_retain(v3), NULL);
    if (v2) still_release(v2);
    v2 = t1;
#line 400 "json"
    still_bp_check(__FILE__, __LINE__);
    if (((v2 == NULL || ((v2)->tag & 0xFF) == (int)'n'))) {
#line 401 "json"
    still_bp_check(__FILE__, __LINE__);
    if (v3) still_release(v3);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
    }
    if (v3) still_release(v3);
    }
#line 402 "json"
    still_bp_check(__FILE__, __LINE__);
    still_any* t2 = (still_any*)still_retain(v2);
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return t2;
    if (v2) still_release(v2);
    if (v1) still_release(v1);
    if (v0) still_release(v0);
    return NULL;
}

int main(int argc, char** argv) {
    still_args_init(argc, argv);
    still_init();
    still_mod_json__init();

#line 3 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 3 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t1 = still_str_new("{\"name\": \"still\", \"n\": 42, \"pi\": 3.5, \"ok\": true, \"xs\": [1, 2, 3], \"sub\": {\"deep\": \"yes\"}, \"nil\": null}", 103);
    still_st_json_u003a__u003a_JsonResult t2 = still_json__parse(t1);
    still_st_json_u003a__u003a_JsonResult_drop(&still_g_main__r);
    still_g_main__r = t2;
#line 4 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 4 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_print_bool(still_g_main__r.ok);
    still_print_sep();
    still_str* t3 = still_str_new("", 0);
    still_print_bool(still_str_cmp(still_g_main__r.error, t3) == 0);
    still_print_nl();
    if (t3) still_release(t3);
#line 5 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
    if (still_g_main__v) still_release(still_g_main__v);
    still_g_main__v = (still_any*)still_retain(still_g_main__r.value);
#line 6 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 6 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t4 = still_str_new("name", 4);
    still_str* t5 = still_str_new("?", 1);
    still_str* t6 = still_json__get_str((still_any*)still_retain(still_g_main__v), t4, t5);
    still_print_str(t6);
    still_print_nl();
    if (t6) still_release(t6);
#line 7 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 7 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t7 = still_str_new("n", 1);
    still_print_int(still_json__get_int((still_any*)still_retain(still_g_main__v), t7, 0));
    still_print_nl();
#line 8 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 8 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t8 = still_str_new("pi", 2);
    still_print_float(still_json__get_float((still_any*)still_retain(still_g_main__v), t8, 0.0));
    still_print_nl();
#line 9 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 9 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t9 = still_str_new("ok", 2);
    still_print_bool(still_json__get_bool((still_any*)still_retain(still_g_main__v), t9, false));
    still_print_nl();
#line 10 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 10 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t10 = still_str_new("xs", 2);
    still_any* t11 = still_json__get_list((still_any*)still_retain(still_g_main__v), t10, NULL);
    if (still_g_main__xs) still_release(still_g_main__xs);
    still_g_main__xs = t11;
#line 11 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 11 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_print_int(still_any_len(still_g_main__xs));
    still_print_sep();
    still_any* t12 = still_any_index_i(still_g_main__xs, 0);
    still_print_any(t12);
    still_print_sep();
    still_any* t13 = still_any_index_i(still_g_main__xs, 2);
    still_print_any(t13);
    still_print_nl();
    if (t13) still_release(t13);
    if (t12) still_release(t12);
#line 12 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 12 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t14 = still_str_new("sub", 3);
    still_any* t15 = still_json__get_dict((still_any*)still_retain(still_g_main__v), t14, NULL);
    if (still_g_main__sub) still_release(still_g_main__sub);
    still_g_main__sub = t15;
#line 13 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 13 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t16 = still_str_new("deep", 4);
    still_str* t17 = still_str_new("no", 2);
    still_str* t18 = still_json__get_str((still_any*)still_retain(still_g_main__sub), t16, t17);
    still_print_str(t18);
    still_print_nl();
    if (t18) still_release(t18);
#line 14 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 14 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t19 = still_str_new("nil", 3);
    still_any* t20 = still_json__get_any((still_any*)still_retain(still_g_main__v), t19, NULL);
    still_print_bool((t20 == NULL || ((t20)->tag & 0xFF) == (int)'n'));
    still_print_nl();
    if (t20) still_release(t20);
#line 15 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 15 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t21 = still_str_new("missing", 7);
    still_str* t22 = still_str_new("fallback", 8);
    still_str* t23 = still_json__get_str((still_any*)still_retain(still_g_main__v), t21, t22);
    still_print_str(t23);
    still_print_nl();
    if (t23) still_release(t23);
#line 16 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 16 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t24 = still_json__stringify((still_any*)still_retain(still_g_main__v));
    still_print_str(t24);
    still_print_nl();
    if (t24) still_release(t24);
#line 17 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 17 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t25 = still_str_new("not json", 8);
    still_print_bool(still_json__valid(t25));
    still_print_nl();
#line 18 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 18 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t26 = still_str_new("{\"a\":}", 6);
    still_st_json_u003a__u003a_JsonResult t27 = still_json__parse(t26);
    still_st_json_u003a__u003a_JsonResult_drop(&still_g_main__bad);
    still_g_main__bad = t27;
#line 19 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 19 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_print_bool(still_g_main__bad.ok);
    still_print_nl();
#line 20 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 20 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t28 = still_str_new("\"a\\nb\\u0041\"", 12);
    still_st_json_u003a__u003a_JsonResult t29 = still_json__parse(t28);
    still_st_json_u003a__u003a_JsonResult_drop(&still_g_main__esc);
    still_g_main__esc = t29;
#line 21 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 21 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_print_any(still_g_main__esc.value);
    still_print_nl();
#line 22 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 22 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t30 = still_json__stringify((still_any*)still_retain(still_g_main__esc.value));
    still_print_str(t30);
    still_print_nl();
    if (t30) still_release(t30);
#line 23 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 23 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t31 = still_str_new("[1.5e2, -7, 0.25]", 17);
    still_st_json_u003a__u003a_JsonResult t32 = still_json__parse(t31);
    still_st_json_u003a__u003a_JsonResult_drop(&still_g_main__nums);
    still_g_main__nums = t32;
#line 24 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 24 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_any* t33 = still_any_index_i(still_g_main__nums.value, 0);
    still_print_any(t33);
    still_print_sep();
    still_any* t34 = still_any_index_i(still_g_main__nums.value, 1);
    still_print_any(t34);
    still_print_sep();
    still_any* t35 = still_any_index_i(still_g_main__nums.value, 2);
    still_print_any(t35);
    still_print_nl();
    if (t35) still_release(t35);
    if (t34) still_release(t34);
    if (t33) still_release(t33);
#line 25 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_bp_check(__FILE__, __LINE__);
#line 25 "/home/z/my-project/still-lang/conformance/valid/json_lib.strc"
    still_str* t36 = still_str_new("{\"a\": [true, false]}", 20);
    still_st_json_u003a__u003a_JsonResult t37 = still_json__parse(t36);
    still_str* t38 = still_json__stringify_pretty((still_any*)still_retain(t37.value), 0);
    still_print_str(t38);
    still_print_nl();
    if (t38) still_release(t38);
    still_st_json_u003a__u003a_JsonResult_drop(&t37);
    return 0;
}

