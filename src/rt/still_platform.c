
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

typedef struct {
    HWND hwnd;
    HDC mem;
    HBITMAP bmp;
    int w, h;
    int open;
    int bg;
    int mouse_x, mouse_y;
    int ldown;
    int click_edge;
    int wheel;
    char input[4096];
    int input_len;
    unsigned char keys[256];
    unsigned char keys_prev[256];
    unsigned char keys_edge[256];
    StilFontSlot fonts[STIL_GUI_FONT_CACHE];
    int font_count;
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
        case WM_PAINT: {
            /* validate AND blit: BeginPaint/EndPaint alone left the window
               unpainted whenever the frame loop stalled (modal move/size
               loops) - the maximize+move black screen */
            PAINTSTRUCT ps;
            HDC dc = BeginPaint(hwnd, &ps);
            if (dc && w && w->mem) {
                BitBlt(dc, 0, 0, w->w, w->h, w->mem, 0, 0, SRCCOPY);
            }
            EndPaint(hwnd, &ps);
            return 0;
        }
        case WM_ERASEBKGND: {
            /* actually paint over the exposed area: returning 1 without
               drawing left stale pixels / bleed-through at the edges */
            HDC hdc = (HDC)wp;
            if (hdc && w && w->mem) {
                BitBlt(hdc, 0, 0, w->w, w->h, w->mem, 0, 0, SRCCOPY);
            }
            return 1;
        }
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

int64_t still_gui_open_window(const char* title, int64_t w, int64_t h) {
    if (w < 16) w = 16;
    if (h < 16) h = 16;
    if (w > 3840) w = 3840;
    if (h > 2160) h = 2160;
    gui_register_class();
    /* 1:1 pixel mapping on high-DPI displays (graceful if missing) */
    typedef void (WINAPI *DpiAwareFn)(void);
    DpiAwareFn dpi = (DpiAwareFn)GetProcAddress(GetModuleHandleA("user32.dll"), "SetProcessDPIAware");
    if (dpi) dpi();

    int slot = -1;
    for (int i = 0; i < STIL_GUI_MAX; i++) {
        if (!g_wins[i].hwnd && !g_wins[i].open) { slot = i; break; }
    }
    if (slot < 0) return 0;

    StilWindow* win = &g_wins[slot];
    memset(win, 0, sizeof(StilWindow));
    win->bg = 0x101418;
    win->open = 1;

    const char* t = (title && title[0]) ? title : "Still";
    RECT r = {0, 0, (LONG)w, (LONG)h};
    AdjustWindowRect(&r, WS_OVERLAPPEDWINDOW & ~(WS_THICKFRAME | WS_MAXIMIZEBOX), FALSE);
    int ww = (int)(r.right - r.left);
    int wh = (int)(r.bottom - r.top);

    HWND hwnd = CreateWindowExA(0, "StillWindow", t,
                                WS_OVERLAPPEDWINDOW & ~(WS_THICKFRAME | WS_MAXIMIZEBOX),
                                CW_USEDEFAULT, CW_USEDEFAULT, ww, wh,
                                NULL, NULL, GetModuleHandleA(NULL), NULL);
    if (!hwnd) {
        win->open = 0;
        return 0;
    }
    win->hwnd = hwnd;
    /* Unicode titles: CreateWindowExA stored raw UTF-8 bytes as ANSI, so an
       em-dash rendered as mojibake. Rewrite through the W API after creation. */
    if (t && t[0]) {
        int wn = MultiByteToWideChar(CP_UTF8, 0, t, -1, NULL, 0);
        if (wn > 0) {
            wchar_t* wname = (wchar_t*)HeapAlloc(GetProcessHeap(), 0, (size_t)wn * sizeof(wchar_t));
            if (wname) {
                if (MultiByteToWideChar(CP_UTF8, 0, t, -1, wname, wn) > 0) {
                    SetWindowTextW(hwnd, wname);
                }
                HeapFree(GetProcessHeap(), 0, wname);
            }
        }
    }
    win->w = (int)w;
    win->h = (int)h;
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    UpdateWindow(hwnd);
    return (int64_t)(uintptr_t)hwnd;
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
    SetWindowTextA(w->hwnd, title ? title : "");
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
    return w ? w->w : 0;
}

int64_t still_gui_height(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return w ? w->h : 0;
}

int64_t still_gui_mouse_x(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return w ? w->mouse_x : -1;
}

int64_t still_gui_mouse_y(int64_t handle) {
    StilWindow* w = gui_win(handle);
    return w ? w->mouse_y : -1;
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
    return w ? w->wheel : 0;
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
    HFONT f = gui_font_lookup(w, size, FW_NORMAL);
    if (f) SelectObject(w->mem, f);
    SetTextColor(w->mem, gui_rgb(rgb));
    wchar_t* wt = still_wide(text);
    if (wt) {
        TextOutW(w->mem, (int)x, (int)y, wt, lstrlenW(wt));
        free(wt);
    }
}

void still_gui_text_bold(int64_t handle, double x, double y, const char* text, int64_t rgb, double size) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !text || !text[0]) return;
    HFONT f = gui_font_lookup(w, size, FW_SEMIBOLD);
    if (f) SelectObject(w->mem, f);
    SetTextColor(w->mem, gui_rgb(rgb));
    wchar_t* wt = still_wide(text);
    if (wt) {
        TextOutW(w->mem, (int)x, (int)y, wt, lstrlenW(wt));
        free(wt);
    }
}

int64_t still_gui_text_width(int64_t handle, const char* text, double size) {
    StilWindow* w = gui_win(handle);
    if (!w || !w->mem || !text || !text[0]) return 0;
    HFONT f = gui_font_lookup(w, size, FW_NORMAL);
    if (f) SelectObject(w->mem, f);
    wchar_t* wt = still_wide(text);
    int width = 0;
    if (wt) {
        SIZE sz;
        if (GetTextExtentPoint32W(w->mem, wt, lstrlenW(wt), &sz)) width = sz.cx;
        free(wt);
    }
    return width;
}

void still_gui_fill_rect(int64_t handle, double x, double y, double w, double h, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
    RECT r = { (LONG)x, (LONG)y, (LONG)(x + w), (LONG)(y + h) };
    HBRUSH b = CreateSolidBrush(gui_rgb(rgb));
    FillRect(win->mem, &r, b);
    DeleteObject(b);
}

void still_gui_fill_round(int64_t handle, double x, double y, double w, double h, double rad, int64_t rgb) {
    StilWindow* win = gui_win(handle);
    if (!win || !win->mem) return;
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
    SaveDC(win->mem);
    IntersectClipRect(win->mem, (int)x, (int)y, (int)(x + w), (int)(y + h));
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
    GpGraphics* g = NULL;
    if (p_GdipCreateFromHDC(win->mem, &g) != 0 || !g) return;
    if (p_GdipSetInterpolationMode) p_GdipSetInterpolationMode(g, 7 /* HighQualityBicubic */);
    if (p_GdipSetSmoothingMode) p_GdipSetSmoothingMode(g, 4 /* AntiAlias */);
    p_GdipDrawImageRectI(g, (GpImage*)(uintptr_t)img, (int)x, (int)y, (int)w, (int)h);
    p_GdipDeleteGraphics(g);
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

#endif /* _WIN32 */

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
