use crate::ast::*;
use crate::check::sanitize;
use crate::span::Span;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

struct LocalSlot {
    cname: String,
    ty: Ty,
    dead: bool,
}

pub struct Emitter {
    prog: Program,
    extern_map: HashMap<String, ExternDecl>,
    helpers: Vec<String>,
    helper_keys: HashSet<String>,
    body: String,
    protos: String,
    globals_c: String,
    tmpc: u32,
    locals: Vec<LocalSlot>,
    temps: Vec<(String, Ty)>,
    temp_set: HashSet<String>,
    loop_local_marks: Vec<usize>,
    loop_temp_marks: Vec<usize>,
    body_local_marks: Vec<usize>,
    moved: HashSet<u32>,
    fn_ret: Ty,
    debug: bool,
    last_line: u32,
    last_file: String,
}

pub fn emit(prog: &Program) -> String {
    let mut extern_map = HashMap::new();
    for e in &prog.externs {
        extern_map.insert(e.name.clone(), e.clone());
    }
    let debug = prog.debug;
    let mut e = Emitter {
        prog: clone_program(prog),
        extern_map,
        helpers: Vec::new(),
        helper_keys: HashSet::new(),
        body: String::new(),
        protos: String::new(),
        globals_c: String::new(),
        tmpc: 0,
        locals: Vec::new(),
        temps: Vec::new(),
        temp_set: HashSet::new(),
        loop_local_marks: Vec::new(),
        loop_temp_marks: Vec::new(),
        body_local_marks: Vec::new(),
        moved: HashSet::new(),
        fn_ret: Ty::NoneTy,
        debug,
        last_line: 0,
        last_file: String::new(),
    };
    e.run();
    let mut out = String::new();
    out.push_str("#include \"still_rt.h\"\n");
    if e.debug {
        out.push_str("#define STIL_DEBUG 1\n");
    }
    out.push_str("#include <math.h>\n");
    for h in &e.helpers {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str(&e.globals_c);
    out.push_str(&e.protos);
    out.push_str(&e.body);
    out
}

fn clone_program(p: &Program) -> Program {
    Program {
        modules: p.modules.clone(),
        funcs: p.funcs.clone(),
        externs: p.externs.clone(),
        structs: p.structs.clone(),
        globals: p.globals.clone(),
        global_order: p.global_order.clone(),
        module_inits: p.module_inits.clone(),
        main_body: p.main_body.clone(),
        debug: p.debug,
    }
}

fn c_escape(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{:03o}", b)),
        }
    }
    out
}

impl Emitter {
    fn run(&mut self) {
        let inits = self.prog.module_inits.clone();
        let mut init_names = Vec::new();
        for (m, _cs) in &inits {
            let name = format!("still_mod_{}__init", sanitize(m));
            init_names.push(name.clone());
            self.protos.push_str(&format!("static void {}(void);\n", name));
        }
        let funcs = self.prog.funcs.clone();
        for f in &funcs {
            let vis = if f.exported { "" } else { "static " };
            let ret = if matches!(resolve(&f.ret), Ty::NoneTy) { "void".to_string() } else { self.ctype(&resolve_deep(&f.ret)) };
            let params: Vec<String> = f
                .params
                .iter()
                .enumerate()
                .map(|(i, (_, t))| format!("{} v{}", self.ctype(&resolve_deep(t)), i))
                .collect();
            self.protos.push_str(&format!("{}{} {}({});\n", vis, ret, f.cname, params.join(", ")));
        }
        for cname in &self.prog.global_order.clone() {
            let g = self.prog.globals[cname].clone();
            let ty = resolve_deep(&g.ty);
            let ct = self.ctype(&ty);
            let z = self.zero_c(&ty);
            self.globals_c.push_str(&format!("static {} {} = {};\n", ct, cname, z));
        }
        let inits2 = self.prog.module_inits.clone();
        for (m, cs) in &inits2 {
            let name = format!("still_mod_{}__init", sanitize(m));
            self.emit_function(&name, true, Ty::NoneTy, &[], cs);
        }
        let funcs2 = self.prog.funcs.clone();
        for f in &funcs2 {
            let params: Vec<(String, Ty)> = f
                .params
                .iter()
                .enumerate()
                .map(|(i, (_, t))| (format!("v{}", i), resolve_deep(t)))
                .collect();
            let ret = resolve_deep(&f.ret);
            self.emit_function(&f.cname, !f.exported, ret, &params, &f.body.clone());
        }
        let main_body = self.prog.main_body.clone();
        let inits3 = init_names.clone();
        let mut head = String::new();
        head.push_str("    still_init();\n");
        for n in &inits3 {
            head.push_str(&format!("    {}();\n", n));
        }
        let save = std::mem::replace(&mut self.body, String::new());
        self.emit_function("main", false, Ty::Int, &[], &main_body);
        let main_code = std::mem::replace(&mut self.body, save);
        self.body.push_str(&main_code.replace("__MODULE_INITS__", &head));
    }

    fn emit_function(&mut self, name: &str, is_static: bool, ret: Ty, params: &[(String, Ty)], body: &[CS]) {
        self.locals.clear();
        self.temps.clear();
        self.temp_set.clear();
        self.moved.clear();
        self.loop_local_marks.clear();
        self.loop_temp_marks.clear();
        self.body_local_marks.clear();
        self.tmpc = 0;
        self.fn_ret = ret.clone();
        self.last_line = 0;
        self.last_file.clear();
        for (cname, ty) in params {
            self.locals.push(LocalSlot { cname: cname.clone(), ty: ty.clone(), dead: false });
        }
        let vis = if is_static && name != "main" { "static " } else { "" };
        let ret_c = if matches!(resolve(&ret), Ty::NoneTy) { "void".to_string() } else { self.ctype(&ret) };
        let param_list: Vec<String> = params
            .iter()
            .map(|(c, t)| format!("{} {}", self.ctype(t), c))
            .collect();
        let pl = if param_list.is_empty() && name != "main" {
            "void".to_string()
        } else if name == "main" {
            "int argc, char** argv".to_string()
        } else {
            param_list.join(", ")
        };
        let ret_c = if name == "main" { "int".to_string() } else { ret_c };
        self.body.push_str(&format!("{}{} {}({}) {{\n", vis, ret_c, name, pl));
        if name == "main" {
            self.body.push_str("    still_args_init(argc, argv);\n__MODULE_INITS__\n");
        }
        let n_params = params.len();
        self.emit_block(body);
        for id in (0..n_params).rev() {
            self.drop_local(id as u32);
        }
        self.drop_temps_to(0);
        if name == "main" {
            self.body.push_str("    return 0;\n");
        } else {
            match resolve(&ret) {
                Ty::NoneTy => self.body.push_str("    return;\n"),
                Ty::Nullable(inner) => {
                    let rd = resolve_deep(&inner);
                    let z = self.zero_c(&rd);
                    let ct = self.ctype(&rd);
                    let z = if z == "{0}" { format!("({}){{0}}", ct) } else { z };
                    self.body.push_str(&format!("    return {};\n", z));
                }
                Ty::Int | Ty::Float | Ty::Bool => {
                    let z = self.zero_c(&resolve(&ret));
                    self.body.push_str(&format!("    return {};\n", z));
                }
                _ => {
                    let rd = resolve_deep(&ret);
                    let z = self.zero_c(&rd);
                    let ct = self.ctype(&rd);
                    let z = if z == "{0}" { format!("({}){{0}}", ct) } else { z };
                    self.body.push_str(&format!("    return {};\n", z));
                }
            }
        }
        self.body.push_str("}\n\n");
    }

    fn line(&mut self, s: String) {
        self.body.push_str("    ");
        self.body.push_str(&s);
        self.body.push('\n');
    }

    fn raw(&mut self, s: &str) {
        self.body.push_str(s);
        self.body.push('\n');
    }

    fn set_line(&mut self, span: &Span) {
        let f = span.file.to_string();
        if span.line != self.last_line || f != self.last_file {
            self.body.push_str(&format!("#line {} \"{}\"\n", span.line, f.replace('\\', "\\\\")));
            self.last_line = span.line;
            self.last_file = f;
        }
    }

    fn bp(&mut self, span: &Span) {
        self.set_line(span);
        if self.debug {
            self.line("still_bp_check(__FILE__, __LINE__);".into());
            self.last_line += 1;
        }
    }

    fn newtmp(&mut self) -> String {
        self.tmpc += 1;
        format!("t{}", self.tmpc)
    }

    fn reg_temp(&mut self, name: &str, ty: Ty) {
        self.temps.push((name.to_string(), ty.clone()));
        self.temp_set.insert(name.to_string());
    }

    fn unreg_temp(&mut self, name: &str) -> Option<Ty> {
        if self.temp_set.remove(name) {
            if let Some(pos) = self.temps.iter().position(|(n, _)| n == name) {
                let (_, ty) = self.temps.remove(pos);
                return Some(ty);
            }
        }
        None
    }

    fn is_temp(&self, expr: &str) -> bool {
        self.temp_set.contains(expr)
    }

    fn drop_temps_to(&mut self, mark: usize) {
        while self.temps.len() > mark {
            let (name, ty) = self.temps.pop().unwrap();
            self.temp_set.remove(&name);
            let d = self.drop_stmt(&ty, &name);
            if !d.is_empty() {
                self.line(d);
            }
        }
    }

    fn drop_local(&mut self, id: u32) {
        if self.moved.contains(&id) || self.locals[id as usize].dead {
            return;
        }
        let (cname, ty) = {
            let s = &self.locals[id as usize];
            (s.cname.clone(), resolve_deep(&s.ty))
        };
        let d = self.drop_stmt(&ty, &cname);
        if !d.is_empty() {
            self.line(d);
        }
    }

    fn drop_stmt(&mut self, ty: &Ty, v: &str) -> String {
        match ty {
            Ty::Str | Ty::List(_) | Ty::Dict(..) => format!("if ({v}) still_release({v});", v = v),
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return format!("if ({v}) still_release({v});", v = v);
                    }
                    if self.struct_has_refs(&si) {
                        return format!("still_st_{}_drop(&{v});", sanitize(q), v = v);
                    }
                }
                String::new()
            }
            Ty::Tuple(ts) => {
                if self.tuple_has_refs(ts) {
                    return format!("still_tup_{}_drop(&{v});", self.tup_tag(ts), v = v);
                }
                String::new()
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                        format!("if ({v}) still_release({v});", v = v)
                    }
                    Ty::Named(ref q) => {
                        let si = self.prog.structs.get(q).cloned();
                        if let Some(si) = si {
                            if si.is_soa {
                                return format!("if ({v}) still_release({v});", v = v);
                            }
                            if self.struct_has_refs(&si) {
                                return format!("if ({v}.has) still_st_{}_drop(&{v}.v);", sanitize(q), v = v);
                            }
                        }
                        String::new()
                    }
                    Ty::Tuple(ref ts) => {
                        if self.tuple_has_refs(ts) {
                            return format!("if ({v}.has) still_tup_{}_drop(&{v}.v);", self.tup_tag(ts), v = v);
                        }
                        String::new()
                    }
                    _ => String::new(),
                }
            }
            _ => String::new(),
        }
    }

    fn struct_has_refs(&self, si: &StructInfo) -> bool {
        si.fields.iter().any(|(_, t)| self.ty_has_refs(&resolve_deep(t)))
    }

    fn tuple_has_refs(&self, ts: &[Ty]) -> bool {
        ts.iter().any(|t| self.ty_has_refs(&resolve_deep(t)))
    }

    fn ty_has_refs(&self, t: &Ty) -> bool {
        match resolve(t) {
            Ty::Str | Ty::List(_) | Ty::Dict(..) => true,
            Ty::Nullable(b) => self.ty_has_refs(&b),
            Ty::Named(q) => self
                .prog
                .structs
                .get(&q)
                .map(|s| s.is_soa || self.struct_has_refs(s))
                .unwrap_or(false),
            Ty::Tuple(ts) => self.tuple_has_refs(&ts),
            _ => false,
        }
    }

    fn zero_c(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Int | Ty::Never => "0".into(),
            Ty::Float => "0.0".into(),
            Ty::Bool => "false".into(),
            Ty::Str | Ty::NoneTy => "NULL".into(),
            Ty::List(_) | Ty::Dict(..) => "NULL".into(),
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => "NULL".into(),
                    _ => format!("{{0}}"),
                }
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return "NULL".into();
                    }
                }
                "{0}".into()
            }
            Ty::Tuple(_) => "{0}".into(),
            Ty::Var(_) => "0".into(),
        }
    }

    pub fn ctype(&mut self, ty: &Ty) -> String {
        match resolve(ty) {
            Ty::Int | Ty::Never => "int64_t".into(),
            Ty::Float => "double".into(),
            Ty::Bool => "bool".into(),
            Ty::Str | Ty::NoneTy => "still_str*".into(),
            Ty::List(e) => {
                let et = resolve_deep(&e);
                let tag = ty_tag(&et);
                self.need_list(&et);
                format!("still_list_{}*", tag)
            }
            Ty::Dict(k, v) => {
                let kt = resolve_deep(&k);
                let vt = resolve_deep(&v);
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                self.need_dict(&kt, &vt);
                format!("still_dict_{}_{}*", ktag, vtag)
            }
            Ty::Tuple(ts) => {
                let t = self.tup_tag(&ts);
                self.need_tuple(&ts);
                format!("still_tup_{}", t)
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => self.ctype(&i),
                    Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false) => self.ctype(&i),
                    _ => {
                        let tag = ty_tag(&i);
                        self.need_opt(&i);
                        format!("still_opt_{}", tag)
                    }
                }
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(&q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        self.need_soa(&q);
                        return format!("still_soa_{}*", si.name);
                    }
                }
                self.need_struct(&q);
                format!("still_st_{}", sanitize(&q))
            }
            Ty::Var(_) => "int64_t".into(),
        }
    }

    fn tup_tag(&mut self, ts: &[Ty]) -> String {
        let parts: Vec<String> = ts.iter().map(|t| ty_tag(&resolve_deep(t))).collect();
        parts.join("_")
    }

    fn eq_fn(&mut self, ty: &Ty) -> String {
        match resolve(ty) {
            Ty::Int | Ty::Never => "still_eq_i64".into(),
            Ty::Float => "still_eq_f64".into(),
            Ty::Bool => "still_eq_bool".into(),
            Ty::Str => "still_str_eq".into(),
            Ty::List(e) => {
                let et = resolve_deep(&e);
                let tag = ty_tag(&et);
                self.need_list(&et);
                format!("still_list_{}_eq", tag)
            }
            Ty::Dict(k, v) => {
                let kt = resolve_deep(&k);
                let vt = resolve_deep(&v);
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                self.need_dict(&kt, &vt);
                format!("still_dict_{}_{}_eq", ktag, vtag)
            }
            Ty::Tuple(ts) => {
                let t = self.tup_tag(&ts);
                self.need_tuple(&ts);
                format!("still_tup_{}_eqv", t)
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                        self.need_eqn(&i);
                        format!("still_eqn_{}", ty_tag(&i))
                    }
                    _ => {
                        self.need_opt(&i);
                        format!("still_opt_{}_eq", ty_tag(&i))
                    }
                }
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(&q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return "still_ptr_eq".into();
                    }
                    self.need_struct(&q);
                    return format!("still_st_{}_eqv", sanitize(&q));
                }
                "still_eq_i64".into()
            }
            Ty::NoneTy => "still_ptr_eq".into(),
            Ty::Var(_) => "still_eq_i64".into(),
        }
    }

    fn need_eqn(&mut self, inner: &Ty) {
        let i = resolve_deep(inner);
        let tag = ty_tag(&i);
        let key = format!("eqn_{}", tag);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let ct = self.ctype(&i);
        let eqf = self.eq_fn(&i);
        self.helpers.push(format!(
            "static bool still_eqn_{tag}(void* a, void* b) {{ return a == b || (a && b && {eqf}(({ct})a, ({ct})b)); }}\n",
            tag = tag,
            eqf = eqf,
            ct = ct
        ));
    }

    fn retain_expr(&mut self, ty: &Ty) -> String {
        let t = resolve_deep(ty);
        match t {
            Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                let ct = self.ctype(&t);
                format!("({ct})still_retain(_v)", ct = ct)
            }
            Ty::Named(ref q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return format!("(still_soa_{}*)still_retain(_v)", si.name);
                    }
                    if self.struct_has_refs(&si) {
                        return format!("still_st_{}_clone(&_v)", sanitize(q));
                    }
                }
                "_v".into()
            }
            Ty::Tuple(ref ts) => {
                if self.tuple_has_refs(ts) {
                    return format!("still_tup_{}_clone(&_v)", self.tup_tag(ts));
                }
                "_v".into()
            }
            Ty::Nullable(ref inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => self.retain_expr(&i),
                    Ty::Named(ref q) => {
                        let si = self.prog.structs.get(q).cloned();
                        if let Some(si) = si {
                            if si.is_soa {
                                return format!("(still_soa_{}*)still_retain(_v)", si.name);
                            }
                            if self.struct_has_refs(&si) {
                                return format!(
                                    "((still_opt_{tag}){{ .v = still_st_{q}_clone(&_v.v), .has = _v.has }})",
                                    tag = ty_tag(&i),
                                    q = sanitize(q)
                                );
                            }
                        }
                        "_v".into()
                    }
                    Ty::Tuple(ref ts) => {
                        if self.tuple_has_refs(ts) {
                            return format!(
                                "((still_opt_{tag}){{ .v = still_tup_{t}_clone(&_v.v), .has = _v.has }})",
                                tag = ty_tag(&i),
                                t = self.tup_tag(ts)
                            );
                        }
                        "_v".into()
                    }
                    _ => "_v".into(),
                }
            }
            _ => "_v".into(),
        }
    }

    fn release_expr(&mut self, ty: &Ty) -> String {
        let t = resolve_deep(ty);
        match t {
            Ty::Str | Ty::List(_) | Ty::Dict(..) => "still_release(_v)".into(),
            Ty::Named(ref q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return "still_release(_v)".into();
                    }
                    if self.struct_has_refs(&si) {
                        return format!("still_st_{}_drop(&_v)", sanitize(q));
                    }
                }
                "((void)_v)".into()
            }
            Ty::Tuple(ref ts) => {
                if self.tuple_has_refs(ts) {
                    return format!("still_tup_{}_drop(&_v)", self.tup_tag(ts));
                }
                "((void)_v)".into()
            }
            Ty::Nullable(ref inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => "still_release(_v)".into(),
                    Ty::Named(ref q) => {
                        let si = self.prog.structs.get(q).cloned();
                        if let Some(si) = si {
                            if si.is_soa {
                                return "still_release(_v)".into();
                            }
                            if self.struct_has_refs(&si) {
                                return format!("if (_v.has) still_st_{}_drop(&_v.v)", sanitize(q));
                            }
                        }
                        "((void)_v)".into()
                    }
                    Ty::Tuple(ref ts) => {
                        if self.tuple_has_refs(ts) {
                            return format!("if (_v.has) still_tup_{}_drop(&_v.v)", self.tup_tag(ts));
                        }
                        "((void)_v)".into()
                    }
                    _ => "((void)_v)".into(),
                }
            }
            _ => "((void)_v)".into(),
        }
    }

    fn clone_expr(&mut self, ty: &Ty) -> String {
        self.retain_expr(ty)
    }

    fn need_list(&mut self, elem: &Ty) {
        let tag = ty_tag(elem);
        if tag == "i" || tag == "s" {
            return;
        }
        let key = format!("list_{}", tag);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let eq = self.eq_fn(elem);
        let retain = self.retain_expr(elem);
        let release = self.release_expr(elem);
        let clone = self.clone_expr(elem);
        let ct = self.ctype(elem);
        self.helpers.push(format!(
            "STILL_DEFINE_LIST({ct}, {tag}, {retain}, {release}, {eq}, {clone})\n",
            ct = ct,
            tag = tag,
            retain = retain,
            release = release,
            eq = eq,
            clone = clone
        ));
    }

    fn need_dict(&mut self, k: &Ty, v: &Ty) {
        let ktag = ty_tag(k);
        let vtag = ty_tag(v);
        let key = format!("dict_{}_{}", ktag, vtag);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let (hashk, eqk, releasek, kct) = match resolve(k) {
            Ty::Int => ("still_hash_i64(_k)", "still_eq_i64", "((void)_k)", "int64_t"),
            _ => ("still_hash_str(_k)", "still_str_eq", "still_release(_k)", "still_str*"),
        };
        let eqv = self.eq_fn(v);
        let releasev = self.release_expr(v);
        let clonev = self.clone_expr(v);
        let vct = self.ctype(v);
        self.helpers.push(format!(
            "STILL_DEFINE_DICT({kct}, {vct}, {ktag}, {vtag}, {hashk}, {eqk}, {eqv}, {releasek}, {releasev}, {clonev})\n",
            kct = kct,
            vct = vct,
            ktag = ktag,
            vtag = vtag,
            hashk = hashk,
            eqk = eqk,
            eqv = eqv,
            releasek = releasek,
            releasev = releasev,
            clonev = clonev
        ));
        self.need_dict_keys_fn(k, v);
    }

    fn need_dict_keys_fn(&mut self, k: &Ty, v: &Ty) {
        let ktag = ty_tag(k);
        let vtag = ty_tag(v);
        let key = format!("dictkeys_{}_{}", ktag, vtag);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        self.need_list(k);
        let tup = vec![k.clone(), v.clone()];
        let ttag = self.tup_tag(&tup);
        let ltag = ty_tag(&Ty::Tuple(Rc::new(tup.clone())));
        self.need_tuple(&tup);
        self.need_list(&Ty::Tuple(Rc::new(tup.clone())));
        let retaink = self.retain_expr(k);
        let retainv = self.retain_expr(v);
        let kct = self.ctype(k);
        let vct = self.ctype(v);
        let code = format!(
            "static still_list_{ktag}* still_dict_{ktag}_{vtag}_keys(still_dict_{ktag}_{vtag}* d) {{\n\
             still_list_{ktag}* r = still_list_{ktag}_alloc(d->len);\n\
             for (int64_t _i = 0; _i < d->cap; _i++) {{\n\
             if (d->slots[_i].state == 1) {{ {kct} _v = d->slots[_i].key; (void)_v; still_list_{ktag}_push(r, {retaink}); }}\n\
             }}\n\
             return r;\n\
             }}\n\
             static still_list_{ltag}* still_dict_{ktag}_{vtag}_pairs(still_dict_{ktag}_{vtag}* d) {{\n\
             still_list_{ltag}* r = still_list_{ltag}_alloc(d->len);\n\
             for (int64_t _i = 0; _i < d->cap; _i++) {{\n\
             if (d->slots[_i].state == 1) {{\n\
             still_tup_{ttag} _p;\n\
             {{ {kct} _v = d->slots[_i].key; (void)_v; _p.f0 = {retaink}; }}\n\
             {{ {vct} _v = d->slots[_i].val; (void)_v; _p.f1 = {retainv}; }}\n\
             still_list_{ltag}_push(r, _p);\n\
             }}\n\
             }}\n\
             return r;\n\
             }}\n",
            ktag = ktag,
            vtag = vtag,
            kct = kct,
            vct = vct,
            retaink = retaink,
            retainv = retainv,
            ttag = ttag,
            ltag = ltag
        );
        self.helpers.push(code);
    }

    fn need_tuple(&mut self, ts: &[Ty]) {
        let t = self.tup_tag(ts);
        let key = format!("tup_{}", t);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let mut def = String::new();
        def.push_str(&format!("typedef struct {{\n"));
        let tys: Vec<Ty> = ts.iter().map(|x| resolve_deep(x)).collect();
        for (i, ty) in tys.iter().enumerate() {
            let ct = self.ctype(ty);
            def.push_str(&format!("    {} f{};\n", ct, i));
        }
        def.push_str(&format!("}} still_tup_{};\n", t));
        if self.tuple_has_refs(&tys) {
            let mut drop = String::new();
            let mut clone = String::new();
            let mut eq = String::new();
            drop.push_str(&format!("static void still_tup_{}_drop(still_tup_{}* v) {{\n", t, t));
            clone.push_str(&format!("static still_tup_{} still_tup_{}_clone(still_tup_{}* v) {{\n    still_tup_{} r = *v;\n", t, t, t, t));
            eq.push_str(&format!("static bool still_tup_{}_eq(still_tup_{}* a, still_tup_{}* b) {{\n    return true", t, t, t));
            for (i, ty) in tys.iter().enumerate() {
                let rel = self.release_expr(ty);
                let ret = self.retain_expr(ty);
                let eqf = self.eq_fn(ty);
                drop.push_str(&format!("    {{ {} _v = v->f{}; (void)_v; {}; }}\n", self.ctype(ty), i, rel));
                clone.push_str(&format!("    {{ {} _v = r.f{}; (void)_v; r.f{} = {}; }}\n", self.ctype(ty), i, i, ret));
                eq.push_str(&format!(" && {}(a->f{}, b->f{})", eqf, i, i));
            }
            drop.push_str("}\n");
            clone.push_str("    return r;\n}\n");
            eq.push_str(";\n}\n");
            def.push_str(&drop);
            def.push_str(&clone);
            def.push_str(&eq);
            def.push_str(&format!(
                "static bool still_tup_{}_eqv(still_tup_{} a, still_tup_{} b) {{ return still_tup_{}_eq(&a, &b); }}\n",
                t, t, t, t
            ));
        } else {
            let mut eq = String::new();
            eq.push_str(&format!("static bool still_tup_{}_eq(still_tup_{}* a, still_tup_{}* b) {{\n    return true", t, t, t));
            for (i, ty) in tys.iter().enumerate() {
                let eqf = self.eq_fn(ty);
                eq.push_str(&format!(" && {}(a->f{}, b->f{})", eqf, i, i));
            }
            eq.push_str(";\n}\n");
            def.push_str(&eq);
            def.push_str(&format!(
                "static bool still_tup_{}_eqv(still_tup_{} a, still_tup_{} b) {{ return still_tup_{}_eq(&a, &b); }}\n",
                t, t, t, t
            ));
            def.push_str(&format!("static void still_tup_{}_drop(still_tup_{}* v) {{ (void)v; }}\n", t, t));
            def.push_str(&format!("static still_tup_{} still_tup_{}_clone(still_tup_{}* v) {{ return *v; }}\n", t, t, t));
        }
        self.helpers.push(def);
    }

    fn need_opt(&mut self, inner: &Ty) {
        let i = resolve_deep(inner);
        let tag = ty_tag(&i);
        let key = format!("opt_{}", tag);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let ct = self.ctype(&i);
        let mut code = format!("typedef struct {{ {} v; bool has; }} still_opt_{};\n", ct, tag);
        code.push_str(&format!("static bool still_opt_{}_eq(still_opt_{} a, still_opt_{} b) {{\n", tag, tag, tag));
        code.push_str(&format!("    if (a.has != b.has) return false;\n"));
        code.push_str(&format!("    if (!a.has) return true;\n"));
        let eqf = self.eq_fn(&i);
        code.push_str(&format!("    return {}(a.v, b.v);\n", eqf));
        code.push_str(&format!("}}\n"));
        self.helpers.push(code);
    }

    fn need_soa(&mut self, qual: &str) {
        let si = self.prog.structs.get(qual).cloned();
        let si = match si {
            Some(s) => s,
            None => return,
        };
        let key = format!("soa_{}", si.name);
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let fields: Vec<(String, Ty)> = si.fields.iter().map(|(n, t)| (n.clone(), resolve_deep(t))).collect();
        let mut code = String::new();
        code.push_str("typedef struct { still_hdr hdr; int64_t len; int64_t cap;");
        for (fname, fty) in &fields {
            let ct = self.ctype(fty);
            code.push_str(&format!(" {}* f_{};", ct, fname));
        }
        code.push_str(&format!(" }} still_soa_{};\n", si.name));
        code.push_str(&format!(
            "static void still_soa_{n}_destroy(void* p) {{ still_soa_{n}* s = (still_soa_{n}*)p;",
            n = si.name
        ));
        for (fname, _) in &fields {
            code.push_str(&format!(" free(s->f_{});", fname));
        }
        code.push_str(" free(p); }\n");
        code.push_str(&format!(
            "static still_soa_{n}* still_soa_{n}_new(int64_t cap) {{\n    still_soa_{n}* s = (still_soa_{n}*)malloc(sizeof(still_soa_{n}));\n    if (!s) still_panic(\"memory\", 0, \"out of memory\");\n    s->hdr.rc = 1;\n    s->hdr.destroy = still_soa_{n}_destroy;\n    s->len = 0;\n    s->cap = cap > 0 ? cap : 8;\n",
            n = si.name
        ));
        for (fname, fty) in &fields {
            let ct = self.ctype(fty);
            code.push_str(&format!("    s->f_{} = ({ct}*)calloc((size_t)s->cap, sizeof({ct}));\n", fname, ct = ct));
        }
        code.push_str("    return s;\n}\n");
        code.push_str(&format!("static void still_soa_{n}_grow(still_soa_{n}* s) {{\n    s->cap *= 2;\n", n = si.name));
        for (fname, fty) in &fields {
            let ct = self.ctype(fty);
            code.push_str(&format!("    {{ {ct}* nd = ({ct}*)realloc(s->f_{f}, (size_t)s->cap * sizeof({ct})); if (!nd) still_panic(\"memory\", 0, \"out of memory\"); s->f_{f} = nd; }}\n", ct = ct, f = fname));
        }
        code.push_str("}\n");
        let mut sig = String::new();
        for (fname, fty) in &fields {
            let ct = self.ctype(fty);
            sig.push_str(&format!("{ct} a_{f}, ", ct = ct, f = fname));
        }
        let sig = sig.trim_end_matches(", ").to_string();
        code.push_str(&format!(
            "static void still_soa_{n}_push(still_soa_{n}* s, {sig}) {{\n    if (s->len == s->cap) still_soa_{n}_grow(s);\n",
            n = si.name,
            sig = sig
        ));
        for (fname, _) in &fields {
            code.push_str(&format!("    s->f_{}[s->len] = a_{};\n", fname, fname));
        }
        code.push_str("    s->len++;\n}\n");
        code.push_str(&format!(
            "static void still_soa_{n}_clear(still_soa_{n}* s) {{ s->len = 0; }}\n",
            n = si.name
        ));
        code.push_str(&format!(
            "static void still_print_soa_{n}(still_soa_{n}* s) {{ if (!s) {{ still_print_none(); return; }} fprintf(stdout, \"{n}(\"); still_print_int(s->len); fputc(')', stdout); }}\n",
            n = si.name
        ));
        self.helpers.push(code);
    }

    fn need_struct(&mut self, qual: &str) {
        let si = self.prog.structs.get(qual).cloned();
        let si = match si {
            Some(s) => s,
            None => return,
        };
        let key = format!("st_{}", sanitize(qual));
        if !self.helper_keys.insert(key.clone()) {
            return;
        }
        let mut code = String::new();
        let cname = format!("still_st_{}", sanitize(qual));
        code.push_str("typedef struct {\n");
        let fields: Vec<(String, Ty)> = si.fields.iter().map(|(n, t)| (n.clone(), resolve_deep(t))).collect();
        for (fname, fty) in &fields {
            let ct = self.ctype(fty);
            code.push_str(&format!("    {} {};\n", ct, fname));
        }
        code.push_str(&format!("}} {};\n", cname));
        if self.struct_has_refs(&si) {
            code.push_str(&format!("static void {c}_drop({c}* v) {{\n", c = cname));
            for (fname, fty) in &fields {
                let rel = self.release_expr(fty);
                code.push_str(&format!("    {{ {} _v = v->{}; (void)_v; {}; }}\n", self.ctype(fty), fname, rel));
            }
            code.push_str("}\n");
            code.push_str(&format!("static {c} {c}_clone({c}* v) {{\n    {c} r = *v;\n", c = cname));
            for (fname, fty) in &fields {
                let ret = self.retain_expr(fty);
                code.push_str(&format!("    {{ {} _v = r.{}; (void)_v; r.{} = {}; }}\n", self.ctype(fty), fname, fname, ret));
            }
            code.push_str("    return r;\n}\n");
        } else {
            code.push_str(&format!("static void {c}_drop({c}* v) {{ (void)v; }}\n", c = cname));
            code.push_str(&format!("static {c} {c}_clone({c}* v) {{ return *v; }}\n", c = cname));
        }
        code.push_str(&format!("static bool {c}_eq({c}* a, {c}* b) {{\n    return true", c = cname));
        for (fname, fty) in &fields {
            let eqf = self.eq_fn(fty);
            code.push_str(&format!(" && {}(a->{f}, b->{f})", eqf, f = fname));
        }
        code.push_str(";\n}\n");
        code.push_str(&format!(
            "static bool {c}_eqv({c} a, {c} b) {{ return {c}_eq(&a, &b); }}\n",
            c = cname
        ));
        self.helpers.push(code);
    }
}

impl Emitter {
    fn emit_block(&mut self, stmts: &[CS]) {
        let mark = self.locals.len();
        for s in stmts {
            self.emit_stmt(s);
        }
        for id in (mark..self.locals.len()).rev() {
            self.drop_local(id as u32);
            self.locals[id].dead = true;
        }
    }

    fn emit_stmt(&mut self, s: &CS) {
        match s {
            CS::Pass => {}
            CS::Decl { local, name, ty, value } => {
                let rty = resolve_deep(ty);
                let cname = format!("v{}", local);
                self.locals.push(LocalSlot { cname: cname.clone(), ty: rty.clone(), dead: false });
                let value = value.clone().unwrap_or(Tx { ty: Ty::NoneTy, span: Span::new(std::rc::Rc::from("d"), 1, 1), x: X::NoneV });
                let vty = resolve_deep(&value.ty);
                self.set_line(&value.span);
                let expr = self.emit_expr(&value);
                let expr = self.adapt_owned(&expr, &vty, &rty);
                if matches!(rty, Ty::NoneTy) {
                    if expr != "0" {
                        self.line(format!("{};", expr));
                    }
                    self.line(format!("still_str* {} = NULL;", cname));
                } else {
                    let ct = self.ctype(&rty);
                    self.line(format!("{} {} = {};", ct, cname, expr));
                }
                self.last_line += 1;
                let _ = name;
            }
            CS::Assign { place, value, ty, span } => {
                self.bp(span);
                let vty = resolve_deep(&value.ty);
                let expr = self.emit_expr(value);
                self.assign_place(place, &resolve_deep(ty), &expr, &vty);
            }
            CS::AugAssign { place, op, value, ty, span } => {
                self.bp(span);
                let read = self.place_read(place);
                let vexpr = self.emit_expr(value);
                let res = self.emit_arith(*op, &read, &resolve_deep(ty), &vexpr, &resolve_deep(&value.ty), span);
                self.assign_place(place, &resolve_deep(ty), &res, &resolve_deep(ty));
            }
            CS::ExprS(tx) => {
                self.bp(&tx.span);
                let e = self.emit_expr(tx);
                if !matches!(e.as_str(), "0" | "false" | "0.0") {
                    self.line(format!("(void)({});", e));
                }
            }
            CS::If { cond, then, els } => {
                self.bp(&cond.span);
                let c = self.emit_expr(cond);
                let t = self.truthy(&resolve_deep(&cond.ty), &c);
                self.line(format!("if ({}) {{", t));
                self.emit_block(then);
                if els.is_empty() {
                    self.raw("    }");
                } else {
                    self.raw("    } else {");
                    self.emit_block(els);
                    self.raw("    }");
                }
                self.last_line += 2;
            }
            CS::While { cond, body } => {
                self.bp(&cond.span);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.raw("    for (;;) {");
                let c = self.emit_expr(cond);
                let t = self.truthy(&resolve_deep(&cond.ty), &c);
                self.line(format!("if (!({})) {{", t));
                self.drop_temps_to(tmark);
                self.last_line += 1;
                self.line("break;".into());
                self.raw("    }");
                self.emit_loop_body(body);
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                self.last_line += 2;
            }
            CS::ForList { list, var, body } => {
                self.bp(&list.span);
                let lty = resolve_deep(&list.ty);
                let elem = match &lty {
                    Ty::List(e) => resolve_deep(e),
                    _ => Ty::Int,
                };
                let l = self.as_var(list);
                let cname = format!("v{}", var);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: cname.clone(), ty: elem.clone(), dead: false });
                self.raw("    for (int64_t __i = 0; __i < {l}->len; __i++) {".replace("{l}", &l).as_str());
                let refd = self.ty_has_refs(&elem);
                if refd {
                    let retain = self.retain_expr(&elem).replace("_v", &format!("{}->data[__i]", l));
                    let ct = self.ctype(&elem);
                    self.line(format!("{} {} = {};", ct, cname, retain));
                } else {
                    let ct = self.ctype(&elem);
                    self.line(format!("{} {} = {}->data[__i];", ct, cname, l));
                }
                self.emit_loop_body(body);
                if refd {
                    let d = self.drop_stmt(&elem, &cname);
                    if !d.is_empty() {
                        self.line(d);
                    }
                }
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                for sl in &mut self.locals {
                    if sl.cname == format!("v{}", var) {
                        sl.dead = true;
                    }
                }
                self.last_line += 2;
            }
            CS::ForRange { start, end, step, var, body } => {
                self.bp(&start.span);
                let s = self.as_var(start);
                let e = self.as_var(end);
                let st = self.as_var(step);
                let cname = format!("v{}", var);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: cname.clone(), ty: Ty::Int, dead: false });
                self.line(format!(
                    "for (int64_t {c} = {s}; ({st} > 0) ? ({c} < {e}) : ({c} > {e}); {c} = still_add({c}, {st}, __FILE__, __LINE__)) {{",
                    c = cname,
                    s = s,
                    e = e,
                    st = st
                ));
                self.emit_loop_body(body);
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                self.last_line += 2;
            }
            CS::ForEnum { list, ivar, evar, body } => {
                self.bp(&list.span);
                let lty = resolve_deep(&list.ty);
                let elem = match &lty {
                    Ty::List(e) => resolve_deep(e),
                    _ => Ty::Int,
                };
                let l = self.as_var(list);
                let icn = format!("v{}", ivar);
                let ecn = format!("v{}", evar);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: icn.clone(), ty: Ty::Int, dead: false });
                self.locals.push(LocalSlot { cname: ecn.clone(), ty: elem.clone(), dead: false });
                self.raw("    for (int64_t __i = 0; __i < {l}->len; __i++) {".replace("{l}", &l).as_str());
                self.line(format!("int64_t {} = __i;", icn));
                let refd = self.ty_has_refs(&elem);
                if refd {
                    let retain = self.retain_expr(&elem).replace("_v", &format!("{}->data[__i]", l));
                    let ct = self.ctype(&elem);
                    self.line(format!("{} {} = {};", ct, ecn, retain));
                } else {
                    let ct = self.ctype(&elem);
                    self.line(format!("{} {} = {}->data[__i];", ct, ecn, l));
                }
                self.emit_loop_body(body);
                if refd {
                    let d = self.drop_stmt(&elem, &ecn);
                    if !d.is_empty() {
                        self.line(d);
                    }
                }
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                for sl in &mut self.locals {
                    if sl.cname == ecn || sl.cname == icn {
                        sl.dead = true;
                    }
                }
                self.last_line += 2;
            }
            CS::ForStr { s, var, body } => {
                self.bp(&s.span);
                let sv = self.as_var(s);
                let cname = format!("v{}", var);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: cname.clone(), ty: Ty::Str, dead: false });
                self.raw("    for (int64_t __i = 0; __i < {s}->len; __i++) {".replace("{s}", &sv).as_str());
                self.line(format!(
                    "still_str* {} = still_str_index({}, __i, __FILE__, __LINE__);",
                    cname, sv
                ));
                self.emit_loop_body(body);
                self.line(format!("if ({}) still_release({});", cname, cname));
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                for sl in &mut self.locals {
                    if sl.cname == cname {
                        sl.dead = true;
                    }
                }
                self.last_line += 2;
            }
            CS::ForSoa { soa, var, body } => {
                self.bp(&soa.span);
                let sv = self.as_var(soa);
                let cname = format!("v{}", var);
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: cname.clone(), ty: Ty::Int, dead: false });
                self.line(format!("for (int64_t {} = 0; {} < {}->len; {}++) {{", cname, cname, sv, cname));
                self.emit_loop_body(body);
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                for sl in &mut self.locals {
                    if sl.cname == cname {
                        sl.dead = true;
                    }
                }
                self.last_line += 2;
            }
            CS::ForPairs { list, avar, bvar, body } => {
                self.bp(&list.span);
                let lty = resolve_deep(&list.ty);
                let (et, _ltag) = match &lty {
                    Ty::List(e) => {
                        let et = resolve_deep(e);
                        let tg = ty_tag(&et);
                        (et, tg)
                    }
                    _ => (Ty::Int, "i".to_string()),
                };
                let l = self.as_var(list);
                let acn = format!("v{}", avar);
                let bcn = format!("v{}", bvar);
                let ta = match &et {
                    Ty::Tuple(ts) => resolve_deep(&ts[0]),
                    _ => Ty::Int,
                };
                let tb = match &et {
                    Ty::Tuple(ts) => resolve_deep(&ts[1]),
                    _ => Ty::Int,
                };
                let tmark = self.temps.len();
                let lmark = self.locals.len();
                self.loop_temp_marks.push(tmark);
                self.loop_local_marks.push(lmark);
                self.locals.push(LocalSlot { cname: acn.clone(), ty: ta.clone(), dead: false });
                self.locals.push(LocalSlot { cname: bcn.clone(), ty: tb.clone(), dead: false });
                self.raw(&format!("    for (int64_t __i = 0; __i < {l}->len; __i++) {{", l = l));
                let access = format!("({l})->data[__i]", l = l);
                let ra = self.retain_expr(&ta).replace("_v", &format!("({access}).f0", access = access));
                let rb = self.retain_expr(&tb).replace("_v", &format!("({access}).f1", access = access));
                let cta = self.ctype(&ta);
                let ctb = self.ctype(&tb);
                self.line(format!("{} {} = {};", cta, acn, ra));
                self.line(format!("{} {} = {};", ctb, bcn, rb));
                self.emit_loop_body(body);
                let da = self.drop_stmt(&ta, &acn);
                if !da.is_empty() {
                    self.line(da);
                }
                let db = self.drop_stmt(&tb, &bcn);
                if !db.is_empty() {
                    self.line(db);
                }
                self.raw("    }");
                self.loop_temp_marks.pop();
                self.loop_local_marks.pop();
                for sl in &mut self.locals {
                    if sl.cname == acn || sl.cname == bcn {
                        sl.dead = true;
                    }
                }
                self.last_line += 2;
            }
            CS::Return { value, span } => {
                self.bp(span);
                let ret = resolve_deep(&self.fn_ret);
                let mut ret_expr: Option<String> = None;
                match value {
                    Some(v) => {
                        let vty = resolve_deep(&v.ty);
                        let expr = self.emit_expr(v);
                        let expr = if self.is_temp(&expr) {
                            self.unreg_temp(&expr);
                            expr
                        } else if expr == "0" || expr == "NULL" || expr.parse::<f64>().is_ok() {
                            expr
                        } else {
                            let owned = self.owned_value(&expr, &vty);
                            let t = self.newtmp();
                            let ct = self.ctype(&vty);
                            self.line(format!("{} {} = {};", ct, t, owned));
                            if self.ty_has_refs(&vty) {
                                self.reg_temp(&t, vty.clone());
                            }
                            t
                        };
                        let expr = self.adapt_ret(&expr, &vty, &ret);
                        self.unreg_temp(&expr);
                        ret_expr = Some(expr);
                    }
                    None => {}
                }
                self.drop_all_locals();
                self.drop_temps_to(0);
                match ret_expr {
                    Some(expr) => {
                        if matches!(ret, Ty::NoneTy) && expr != "0" {
                            self.line(format!("{};", expr));
                            self.line("return;".into());
                        } else {
                            self.line(format!("return {};", expr));
                        }
                    }
                    None => match resolve(&ret) {
                        Ty::NoneTy => self.line("return;".into()),
                        _ => {
                            let z = self.zero_c(&ret);
                    self.line(format!("return {};", z));
                        }
                    },
                }
                self.last_line += 1;
            }
            CS::Break(_) => {
                self.drop_temps_to(*self.loop_temp_marks.last().unwrap_or(&0));
                let lmark = *self.loop_local_marks.last().unwrap_or(&0);
                for id in (lmark..self.locals.len()).rev() {
                    self.drop_local(id as u32);
                }
                self.line("break;".into());
                self.last_line += 1;
            }
            CS::Continue(_) => {
                self.drop_temps_to(*self.loop_temp_marks.last().unwrap_or(&0));
                let lmark = *self.loop_local_marks.last().unwrap_or(&0);
                for id in (lmark..self.locals.len()).rev() {
                    self.drop_local(id as u32);
                }
                self.line("continue;".into());
                self.last_line += 1;
            }
        }
        let m = self.body_local_marks.last().copied().unwrap_or(0);
        self.drop_temps_to(m);
    }

    fn drop_all_locals(&mut self) {
        for id in (0..self.locals.len()).rev() {
            self.drop_local(id as u32);
        }
    }

    fn emit_loop_body(&mut self, body: &[CS]) {
        let tmark = self.temps.len();
        self.body_local_marks.push(tmark);
        self.emit_block(body);
        self.drop_temps_to(tmark);
        self.body_local_marks.pop();
    }

    fn release_stmt_expr(&mut self, ty: &Ty, v: &str) -> String {
        let r = self.release_expr(ty);
        if r == "((void)_v)" {
            return String::new();
        }
        r.replace("_v", v)
    }

    fn as_var(&mut self, tx: &Tx) -> String {
        let expr = self.emit_expr(tx);
        if self.locals.iter().any(|l| l.cname == expr) || self.temp_set.contains(&expr) || self.prog.globals.contains_key(&expr) {
            return expr;
        }
        let ty = resolve_deep(&tx.ty);
        let t = self.newtmp();
        let ct = self.ctype(&ty);
        self.line(format!("{} {} = {};", ct, t, expr));
        if self.ty_has_refs(&ty) {
            self.reg_temp(&t, ty.clone());
        }
        t
    }

    fn place_lvalue(&mut self, p: &Place) -> String {
        match p {
            Place::Local(id) => self.locals[*id as usize].cname.clone(),
            Place::Global(c) => c.clone(),
            Place::Field(base, name) => format!("{}.{}", self.place_lvalue(base), name),
            Place::Index(base, idx) => format!("{}[{}]", self.place_deref_base(base), self.emit_expr(idx)),
            Place::SoaElem(base, name, idx) => {
                let b = self.place_lvalue(base);
                let i = self.emit_expr(idx);
                format!("{}->f_{}[still_soa_index({}, {}->len, __FILE__, __LINE__)]", b, name, i, b)
            }
            Place::TupleIndex(base, i) => format!("{}.f{}", self.place_lvalue(base), i),
        }
    }

    fn place_deref_base(&mut self, p: &Place) -> String {
        match p {
            Place::Local(id) => {
                let s = self.locals[*id as usize].cname.clone();
                let ty = resolve_deep(&self.locals[*id as usize].ty);
                match ty {
                    Ty::List(_) => format!("{}->data", s),
                    _ => s,
                }
            }
            Place::Global(c) => {
                let ty = self.prog.globals.get(c).map(|g| resolve_deep(&g.ty)).unwrap_or(Ty::Int);
                match ty {
                    Ty::List(_) => format!("{}->data", c),
                    _ => c.clone(),
                }
            }
            Place::Field(base, name) => format!("{}.{}", self.place_lvalue(base), name),
            Place::Index(base, idx) => format!("{}[{}]", self.place_deref_base(base), self.emit_expr(idx)),
            Place::SoaElem(base, name, idx) => self.place_lvalue(&Place::SoaElem(base.clone(), name.clone(), idx.clone())),
            Place::TupleIndex(base, i) => format!("{}.f{}", self.place_lvalue(base), i),
        }
    }

    fn place_base_ty(&self, p: &Place) -> Ty {
        match p {
            Place::Local(id) => self.locals[*id as usize].ty.clone(),
            Place::Global(c) => self.prog.globals.get(c).map(|g| g.ty.clone()).unwrap_or(Ty::Int),
            Place::Field(base, name) => {
                let bt = resolve_deep(&self.place_base_ty(base));
                if let Ty::Named(q) = &bt {
                    if let Some(si) = self.prog.structs.get(q) {
                        for (n, t) in &si.fields {
                            if n == name {
                                return t.clone();
                            }
                        }
                    }
                }
                Ty::Int
            }
            Place::Index(base, _) => {
                let bt = resolve_deep(&self.place_base_ty(base));
                match bt {
                    Ty::List(e) => (*e).clone(),
                    Ty::Dict(_, v) => (*v).clone(),
                    _ => Ty::Int,
                }
            }
            Place::SoaElem(base, name, _) => {
                let bt = resolve_deep(&self.place_base_ty(base));
                if let Ty::Named(q) = &bt {
                    if let Some(si) = self.prog.structs.get(q) {
                        for (n, t) in &si.fields {
                            if n == name {
                                return t.clone();
                            }
                        }
                    }
                }
                Ty::Int
            }
            Place::TupleIndex(base, i) => {
                let bt = resolve_deep(&self.place_base_ty(base));
                if let Ty::Tuple(ts) = &bt {
                    if (*i as usize) < ts.len() {
                        return ts[*i as usize].clone();
                    }
                }
                Ty::Int
            }
        }
    }

    fn place_read(&mut self, p: &Place) -> String {
        match p {
            Place::Index(base, idx) => {
                let bty = resolve_deep(&self.place_base_ty(base));
                let b = self.place_lvalue(base);
                let i = self.emit_expr(idx);
                match bty {
                    Ty::List(e) => {
                        let tag = ty_tag(&resolve_deep(&e));
                        format!("still_list_{}_get({}, {}, __FILE__, __LINE__)", tag, b, i)
                    }
                    Ty::Dict(k, v) => {
                        format!(
                            "still_dict_{}_{}_get({}, {}, __FILE__, __LINE__)",
                            ty_tag(&resolve_deep(&k)),
                            ty_tag(&resolve_deep(&v)),
                            b,
                            i
                        )
                    }
                    _ => format!("{}[{}]", b, i),
                }
            }
            Place::SoaElem(..) => self.place_lvalue(p),
            _ => self.place_lvalue(p),
        }
    }

    fn old_release(&mut self, place: &Place, ty: &Ty) {
        let t = resolve_deep(ty);
        match &t {
            Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                let pl = self.place_lvalue(place);
                self.line(format!("if ({}) still_release({});", pl, pl));
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        let pl = self.place_lvalue(place);
                        self.line(format!("if ({}) still_release({});", pl, pl));
                    } else if self.struct_has_refs(&si) {
                        let pl = self.place_lvalue(place);
                        self.line(format!("still_st_{}_drop(&{});", sanitize(q), pl));
                    }
                }
            }
            Ty::Tuple(ts) => {
                if self.tuple_has_refs(ts) {
                    let pl = self.place_lvalue(place);
                    let __h0 = self.tup_tag(ts);
                    self.line(format!("still_tup_{}_drop(&{});", __h0, pl));
                }
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                        let pl = self.place_lvalue(place);
                        self.line(format!("if ({}) still_release({});", pl, pl));
                    }
                    Ty::Named(ref q) => {
                        let si = self.prog.structs.get(q).cloned();
                        if let Some(si) = si {
                            if si.is_soa {
                                let pl = self.place_lvalue(place);
                                self.line(format!("if ({}) still_release({});", pl, pl));
                            } else if self.struct_has_refs(&si) {
                                let pl = self.place_lvalue(place);
                                self.line(format!("if ({}.has) still_st_{}_drop(&{}.v);", pl, sanitize(q), pl));
                            }
                        }
                    }
                    Ty::Tuple(ref ts) => {
                        if self.tuple_has_refs(ts) {
                            let pl = self.place_lvalue(place);
                            let __h0 = self.tup_tag(ts);
                            self.line(format!("if ({}.has) still_tup_{}_drop(&{}.v);", pl, __h0, pl));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn assign_place(&mut self, place: &Place, pty: &Ty, expr: &str, vty: &Ty) {
        let pty = resolve_deep(pty);
        match place {
            Place::Index(base, idx) => {
                let bty = resolve_deep(&self.place_base_ty(base));
                let b = self.place_lvalue(base);
                let i = self.emit_expr(idx);
                let val = self.adapt_owned(expr, &resolve_deep(vty), &pty);
                match &bty {
                    Ty::List(e) => {
                        let tag = ty_tag(&resolve_deep(e));
                        self.line(format!("still_list_{}_set({}, {}, {}, __FILE__, __LINE__);", tag, b, i, val));
                    }
                    Ty::Dict(k, v) => {
                        let key = self.adapt_owned(&i, &Ty::Int, &resolve_deep(&k));
                        let bvar = self.materialize_place_read(base, &bty);
                        self.line(format!("still_dict_{}_{}_set({}, {}, {});", ty_tag(&resolve_deep(&k)), ty_tag(&resolve_deep(&v)), bvar, key, val));
                    }
                    _ => {
                        self.line(format!("{}[{}] = {};", b, i, val));
                    }
                }
                return;
            }
            _ => {}
        }
        let val = self.adapt_owned(expr, &resolve_deep(vty), &pty);
        if matches!(pty, Ty::NoneTy) {
            if val != "0" && val != "NULL" {
                self.line(format!("{};", val));
            }
            let pl = self.place_lvalue(place);
            self.line(format!("{} = NULL;", pl));
            return;
        }
        if self.ty_has_refs(&pty) || matches!(pty, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
            self.old_release(place, &pty);
        }
        let pl = self.place_lvalue(place);
        let ct = self.ctype(&pty);
        let val = if val == "{0}" { format!("({}){{0}}", ct) } else { val };
        self.line(format!("{} = {};", pl, val));
        self.last_line += 1;
    }

    fn adapt_owned(&mut self, expr: &str, from: &Ty, to: &Ty) -> String {
        let f = resolve_deep(from);
        let t = resolve_deep(to);
        if matches!(t, Ty::NoneTy) {
            return expr.to_string();
        }
        match (&f, &t) {
            (Ty::Never, _) => expr.to_string(),
            (Ty::NoneTy, _) => {
                if expr != "0" && expr != "NULL" {
                    self.line(format!("{};", expr));
                }
                self.zero_c(&t)
            }
            (Ty::Int, Ty::Float) => format!("(double)({})", expr),
            (Ty::Str, Ty::Nullable(inner)) if matches!(**inner, Ty::Str) => {
                if self.is_temp(expr) {
                    self.unreg_temp(expr);
                    expr.to_string()
                } else {
                    format!("(still_str*)still_retain({})", expr)
                }
            }
            (_, Ty::Nullable(inner)) => {
                let inner = resolve_deep(inner);
                if ty_eq(&f, &inner) {
                    let owned = self.owned_value(expr, &f);
                    match inner {
                        Ty::Str | Ty::List(_) | Ty::Dict(..) => owned,
                        Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false) => owned,
                        _ => format!("((still_opt_{tag}){{ .v = {owned}, .has = 1 }})", tag = ty_tag(&inner), owned = owned),
                    }
                } else {
                    expr.to_string()
                }
            }
            _ => {
                if self.ty_has_refs(&t) || matches!(t, Ty::Tuple(_) | Ty::Named(_)) {
                    self.owned_value(expr, &t)
                } else {
                    expr.to_string()
                }
            }
        }
    }

    fn owned_value(&mut self, expr: &str, ty: &Ty) -> String {
        if self.is_temp(expr) {
            self.unreg_temp(expr);
            return expr.to_string();
        }
        let t = resolve_deep(ty);
        let r = self.retain_expr(&t);
        if r == "_v" {
            return expr.to_string();
        }
        r.replace("_v", expr)
    }
}

impl Emitter {
    fn adapt_ret(&mut self, expr: &str, from: &Ty, to: &Ty) -> String {
        let f = resolve_deep(from);
        let t = resolve_deep(to);
        match (&f, &t) {
            (Ty::Never, _) => expr.to_string(),
            (Ty::NoneTy, _) => {
                if expr != "0" && expr != "NULL" {
                    self.line(format!("{};", expr));
                }
                self.zero_c(&t)
            }
            (Ty::Int, Ty::Float) => format!("(double)({})", expr),
            _ => expr.to_string(),
        }
    }

    fn materialize_place_read(&mut self, base: &Place, bty: &Ty) -> String {
        if matches!(base, Place::Local(_) | Place::Global(_)) {
            return self.place_lvalue(base);
        }
        let t = self.newtmp();
        let read = self.place_read(base);
        let ct = self.ctype(&resolve_deep(bty));
        self.line(format!("{} {} = {};", ct, t, read));
        let bt = resolve_deep(bty);
        if self.ty_has_refs(&bt) || matches!(bt, Ty::Tuple(_) | Ty::Named(_)) {
            self.reg_temp(&t, bt);
        }
        t
    }

    fn truthy(&mut self, ty: &Ty, e: &str) -> String {
        match resolve(ty) {
            Ty::Int | Ty::Never => format!("(({}) != 0)", e),
            Ty::Float => format!("(({}) != 0.0)", e),
            Ty::Bool => format!("({})", e),
            Ty::Str => format!("(({}) && ({})->len > 0)", e, e),
            Ty::List(_) | Ty::Dict(..) => format!("(({}) && ({})->len > 0)", e, e),
            Ty::Named(q) => {
                if self.prog.structs.get(&q).map(|s| s.is_soa).unwrap_or(false) {
                    format!("(({}) && ({})->len > 0)", e, e)
                } else {
                    "true".into()
                }
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(&inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => format!("(({}) != NULL)", e),
                    Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false) => format!("(({}) != NULL)", e),
                    _ => format!("(({}).has)", e),
                }
            }
            Ty::Tuple(_) => "true".into(),
            Ty::NoneTy => "false".into(),
            Ty::Var(_) => format!("(({}) != 0)", e),
        }
    }

    fn cmp_op_str(op: CmpOp) -> &'static str {
        match op {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        }
    }

    fn emit_arith(&mut self, op: BinOp, l: &str, lty: &Ty, r: &str, rty: &Ty, span: &Span) -> String {
        self.set_line(span);
        let lt = resolve_deep(lty);
        let rt = resolve_deep(rty);
        match (&lt, &rt) {
            (Ty::Int, Ty::Int) => match op {
                BinOp::Add => format!("still_add({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::Sub => format!("still_sub({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::Mul => format!("still_mul({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::FloorDiv => format!("still_floordiv({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::Mod => format!("still_imod({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::Pow => format!("still_ipow({}, {}, __FILE__, __LINE__)", l, r),
                BinOp::Shl => format!("(({}) << ({}))", l, r),
                BinOp::Shr => format!("(({}) >> ({}))", l, r),
                BinOp::Band => format!("(({}) & ({}))", l, r),
                BinOp::Bor => format!("(({}) | ({}))", l, r),
                BinOp::Bxor => format!("(({}) ^ ({}))", l, r),
                BinOp::Div => format!("({} / (double){})", l, r),
            },
            (Ty::Float, _) | (_, Ty::Float) => match op {
                BinOp::Add => format!("({} + {})", l, r),
                BinOp::Sub => format!("({} - {})", l, r),
                BinOp::Mul => format!("({} * {})", l, r),
                BinOp::Div => format!("({} / {})", l, r),
                BinOp::FloorDiv => format!("still_floordivf({}, {})", l, r),
                BinOp::Mod => format!("still_fmodf_({}, {})", l, r),
                BinOp::Pow => format!("pow({}, {})", l, r),
                BinOp::Shl => format!("(({}) << ({}))", l, r),
                BinOp::Shr => format!("(({}) >> ({}))", l, r),
                BinOp::Band => format!("(({}) & ({}))", l, r),
                BinOp::Bor => format!("(({}) | ({}))", l, r),
                BinOp::Bxor => format!("(({}) ^ ({}))", l, r),
            },
            (Ty::Str, Ty::Str) => {
                let t = self.newtmp();
                self.line(format!("still_str* {} = still_str_cat({}, {});", t, l, r));
                self.reg_temp(&t, Ty::Str);
                t
            }
            (Ty::List(e), Ty::List(_)) => {
                let tag = ty_tag(&resolve_deep(e));
                self.need_list(&resolve_deep(e));
                let t = self.newtmp();
                self.line(format!("still_list_{}* {} = still_list_{}_concat({}, {});", tag, t, tag, l, r));
                self.reg_temp(&t, Ty::List(e.clone()));
                t
            }
            _ => format!("({} + {})", l, r),
        }
    }

    fn float_lit(v: f64) -> String {
        if v.is_nan() {
            "(0.0 / 0.0)".into()
        } else if v.is_infinite() {
            if v > 0.0 { "1e999".into() } else { "(-1e999)".into() }
        } else if v == v.trunc() && v.abs() < 1e15 {
            format!("{:.1}", v)
        } else {
            format!("{}", v)
        }
    }

    fn emit_expr(&mut self, tx: &Tx) -> String {
        let sp = tx.span.clone();
        match &tx.x {
            X::Int(v) => format!("{}", v),
            X::Float(v) => Emitter::float_lit(*v),
            X::Bool(v) => format!("{}", v),
            X::NoneV => "0".into(),
            X::Str(s) => {
                self.set_line(&sp);
                let t = self.newtmp();
                self.line(format!(
                    "still_str* {} = still_str_new(\"{}\", {});",
                    t,
                    c_escape(s),
                    s.len()
                ));
                self.reg_temp(&t, Ty::Str);
                t
            }
            X::Local(id) => self.locals[*id as usize].cname.clone(),
            X::Global(c) => c.clone(),
            X::ListLit(items) => {
                let ety = match resolve_deep(&tx.ty) {
                    Ty::List(e) => resolve_deep(&e),
                    _ => Ty::Int,
                };
                self.need_list(&ety);
                let tag = ty_tag(&ety);
                let t = self.newtmp();
                self.set_line(&sp);
                self.line(format!("still_list_{}* {} = still_list_{}_alloc({});", tag, t, tag, items.len()));
                self.reg_temp(&t, Ty::List(Rc::new(ety.clone())));
                let mut vals = Vec::new();
                for it in items {
                    let e = self.emit_expr(it);
                    let vty = resolve_deep(&it.ty);
                    vals.push(self.adapt_owned(&e, &vty, &ety));
                }
                for v in vals {
                    self.line(format!("still_list_{}_push({}, {});", tag, t, v));
                }
                t
            }
            X::DictLit(pairs) => {
                let (kt, vt) = match resolve_deep(&tx.ty) {
                    Ty::Dict(k, v) => (resolve_deep(&k), resolve_deep(&v)),
                    _ => (Ty::Int, Ty::Int),
                };
                self.need_dict(&kt, &vt);
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                let t = self.newtmp();
                self.set_line(&sp);
                self.line(format!(
                    "still_dict_{}_{}* {} = still_dict_{}_{}_alloc();",
                    ktag, vtag, t, ktag, vtag
                ));
                self.reg_temp(&t, Ty::Dict(Rc::new(kt.clone()), Rc::new(vt.clone())));
                let mut kvs = Vec::new();
                for (k, v) in pairs {
                    let ke = self.emit_expr(k);
                    let ve = self.emit_expr(v);
                    let kty = resolve_deep(&k.ty);
                    let vty = resolve_deep(&v.ty);
                    let kc = self.adapt_owned(&ke, &kty, &kt);
                    let vc = self.adapt_owned(&ve, &vty, &vt);
                    kvs.push((kc, vc));
                }
                for (kc, vc) in kvs {
                    self.line(format!("still_dict_{}_{}_set({}, {}, {});", ktag, vtag, t, kc, vc));
                }
                t
            }
            X::TupleLit(items) => {
                let tys: Vec<Ty> = items.iter().map(|i| resolve_deep(&i.ty)).collect();
                let tag = self.tup_tag(&tys);
                self.need_tuple(&tys);
                let t = self.newtmp();
                self.set_line(&sp);
                let mut vals = Vec::new();
                for it in items {
                    let e = self.emit_expr(it);
                    let vty = resolve_deep(&it.ty);
                    vals.push(self.adapt_owned(&e, &vty, &resolve_deep(&vty)));
                }
                let inits = vals.join(", ");
                self.line(format!("still_tup_{} {} = {{ {} }};", tag, t, inits));
                if self.tuple_has_refs(&tys) {
                    self.reg_temp(&t, Ty::Tuple(Rc::new(tys)));
                }
                t
            }
            X::StructLit(qual, fields) => {
                self.need_struct(qual);
                let cname = format!("still_st_{}", sanitize(qual));
                let si = self.prog.structs.get(qual).cloned();
                let t = self.newtmp();
                self.set_line(&sp);
                let mut inits: Vec<String> = Vec::new();
                let mut field_tys = Vec::new();
                for (fname, fv) in fields {
                    let e = self.emit_expr(fv);
                    let fty = si
                        .as_ref()
                        .and_then(|s| s.fields.iter().find(|(n, _)| n == fname).map(|(_, t)| resolve_deep(t)))
                        .unwrap_or(Ty::Int);
                    let vty = resolve_deep(&fv.ty);
                    inits.push(format!(".{} = {}", fname, self.adapt_owned(&e, &vty, &fty)));
                    field_tys.push(fty);
                }
                self.line(format!("{} {} = {{ {} }};", cname, t, inits.join(", ")));
                let has_refs = field_tys.iter().any(|f| self.ty_has_refs(f));
                if has_refs {
                    self.reg_temp(&t, Ty::Named(qual.clone()));
                }
                t
            }
            X::SoaNew(qual, cap) => {
                self.need_soa(qual);
                let si = self.prog.structs.get(qual).cloned();
                let name = si.map(|s| s.name).unwrap_or_default();
                let c = self.emit_expr(cap);
                let t = self.newtmp();
                self.set_line(&sp);
                self.line(format!("still_soa_{}* {} = still_soa_{}_new({});", name, t, name, c));
                self.reg_temp(&t, Ty::Named(qual.clone()));
                t
            }
            X::Neg(v) => {
                let e = self.emit_expr(v);
                format!("(-({}))", e)
            }
            X::BNot(v) => {
                let e = self.emit_expr(v);
                format!("(~({}))", e)
            }
            X::Not(v) => {
                let e = self.emit_expr(v);
                let t = self.truthy(&resolve_deep(&v.ty), &e);
                format!("(!{})", t)
            }
            X::Arith(op, l, r) => {
                let le = self.emit_expr(l);
                let re = self.emit_expr(r);
                self.emit_arith(*op, &le, &l.ty, &re, &r.ty, &sp)
            }
            X::And(l, r) => {
                let le = self.emit_expr(l);
                let re = self.emit_expr(r);
                self.emit_cond_value(&le, &resolve_deep(&l.ty), &re, &resolve_deep(&r.ty), &resolve_deep(&tx.ty), true)
            }
            X::Or(l, r) => {
                let le = self.emit_expr(l);
                let re = self.emit_expr(r);
                self.emit_cond_value(&le, &resolve_deep(&l.ty), &re, &resolve_deep(&r.ty), &resolve_deep(&tx.ty), false)
            }
            X::Cmp(op, l, r) => {
                let le = self.emit_expr(l);
                let re = self.emit_expr(r);
                self.emit_cmp(*op, &le, &resolve_deep(&l.ty), &re, &resolve_deep(&r.ty), &sp)
            }
            X::In(item, cont, neg) => {
                let ie = self.emit_expr(item);
                let ce = self.emit_expr(cont);
                let cty = resolve_deep(&cont.ty);
                let cond = match &cty {
                    Ty::List(e) => {
                        let tag = ty_tag(&resolve_deep(e));
                        self.need_list(&resolve_deep(e));
                        format!("still_list_{}_contains({}, {})", tag, ce, ie)
                    }
                    Ty::Dict(k, _) => format!(
                        "still_dict_{}_{}_has({}, {})",
                        ty_tag(&resolve_deep(k)),
                        ty_tag(&resolve_deep(&_dummy_v(&cty))),
                        ce,
                        ie
                    ),
                    Ty::Str => format!("still_str_contains({}, {})", ce, ie),
                    _ => "false".into(),
                };
                if *neg {
                    format!("(!{})", cond)
                } else {
                    cond
                }
            }
            X::IsNone(v, neg) => {
                let e = self.emit_expr(v);
                let vty = resolve_deep(&v.ty);
                let cond = match &vty {
                    Ty::Nullable(inner) => {
                        let i = resolve_deep(inner);
                        match i {
                            Ty::Str | Ty::List(_) | Ty::Dict(..) => format!("({} == NULL)", e),
                            Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false) => format!("({} == NULL)", e),
                            _ => format!("(!{}.has)", e),
                        }
                    }
                    _ => format!("({} == NULL)", e),
                };
                if *neg {
                    format!("(!{})", cond)
                } else {
                    cond
                }
            }
            X::Cond(c, t, e) => {
                let ce = self.emit_expr(c);
                let te = self.emit_expr(t);
                let ee = self.emit_expr(e);
                let ct = self.truthy(&resolve_deep(&c.ty), &ce);
                let rty = resolve_deep(&tx.ty);
                if self.ty_has_refs(&rty) || matches!(rty, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                    let tvar = self.newtmp();
                    self.set_line(&sp);
                    let __h0 = self.ctype(&rty);
                    self.line(format!("{} {};", __h0, tvar));
                    self.line(format!("if ({}) {{", ct));
                    let tv = self.owned_value(&te, &resolve_deep(&t.ty));
                    self.line(format!("{} = {};", tvar, tv));
                    self.rel_if_owned(&ee, &resolve_deep(&e.ty));
                    self.raw("    } else {");
                    let ev = self.owned_value(&ee, &resolve_deep(&e.ty));
                    self.line(format!("{} = {};", tvar, ev));
                    self.rel_if_owned(&te, &resolve_deep(&t.ty));
                    self.raw("    }");
                    self.reg_temp(&tvar, rty.clone());
                    tvar
                } else {
                    format!("({} ? {} : {})", ct, te, ee)
                }
            }
            X::CallFn(idx, args) => {
                let f = self.prog.funcs[*idx].clone();
                let mut adapted = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let e = self.emit_expr(a);
                    let pty = resolve_deep(&f.params[i].1);
                    let aty = resolve_deep(&a.ty);
                    adapted.push(self.adapt_owned(&e, &aty, &pty));
                }
                let ret = resolve_deep(&f.ret);
                let call = format!("{}({})", f.cname, adapted.join(", "));
                if self.ty_has_refs(&ret) || matches!(ret, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                    if matches!(ret, Ty::NoneTy) {
                        call
                    } else {
                        let t = self.newtmp();
                        self.set_line(&sp);
                        let __h0 = self.ctype(&ret);
                        self.line(format!("{} {} = {};", __h0, t, call));
                        self.reg_temp(&t, ret.clone());
                        t
                    }
                } else if matches!(ret, Ty::NoneTy) {
                    call
                } else {
                    call
                }
            }
            X::CallExtern(name, args) => {
                let ed = self.extern_map.get(name).cloned();
                let mut adapted = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let e = self.emit_expr(a);
                    let aty = resolve_deep(&a.ty);
                    let want: Option<Ty> = ed.as_ref().and_then(|d| d.c_params.get(i).cloned());
                    match want {
                        Some(Ty::Str) => {
                            if self.is_temp(&e) {
                                adapted.push(format!("{}->bytes", e));
                            } else if matches!(aty, Ty::Str) {
                                adapted.push(format!("{}->bytes", e));
                            } else {
                                adapted.push(e);
                            }
                        }
                        Some(Ty::Float) if matches!(aty, Ty::Int) => adapted.push(format!("(double)({})", e)),
                        _ => {
                            if matches!(aty, Ty::Str) && ed.is_some() && i >= ed.as_ref().unwrap().c_params.len() {
                                adapted.push(format!("{}->bytes", e));
                            } else if matches!(aty, Ty::Bool) && ed.is_some() && i >= ed.as_ref().unwrap().c_params.len() {
                                adapted.push(format!("(int)({})", e));
                            } else {
                                adapted.push(e);
                            }
                        }
                    }
                }
                let ret = ed.as_ref().map(|d| resolve_deep(&d.c_ret)).unwrap_or(Ty::NoneTy);
                let call = format!("{}({})", name, adapted.join(", "));
                if matches!(ret, Ty::Str) {
                    // C functions that "return str" hand back a borrowed
                    // NUL-terminated const char*. Copy it into a retained
                    // still_str exactly once (never call the C function a
                    // second time just to take strlen). The pointer temp is
                    // borrowed, not owned, so it is not reg_temp'ed.
                    let p = self.newtmp();
                    self.set_line(&sp);
                    self.line(format!("const char* {} = {};", p, call));
                    let t = self.newtmp();
                    self.line(format!(
                        "still_str* {} = still_str_new({}, (int64_t)strlen({}));",
                        t, p, p
                    ));
                    self.reg_temp(&t, Ty::Str);
                    t
                } else if self.ty_has_refs(&ret) {
                    let t = self.newtmp();
                    self.set_line(&sp);
                    let __h0 = self.ctype(&ret);
                    self.line(format!("{} {} = {};", __h0, t, call));
                    self.reg_temp(&t, ret.clone());
                    t
                } else {
                    call
                }
            }
            X::Index(obj, idx) => {
                let oe = self.emit_expr(obj);
                let ie = self.emit_expr(idx);
                let oty = resolve_deep(&obj.ty);
                match &oty {
                    Ty::List(e) => {
                        let et = resolve_deep(e);
                        let tag = ty_tag(&et);
                        self.need_list(&et);
                        let g = format!("still_list_{}_get({}, {}, __FILE__, __LINE__)", tag, oe, ie);
                        self.set_line(&sp);
                        if self.ty_has_refs(&et) || matches!(et, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                            let t = self.newtmp();
                            let __h0 = self.ctype(&et);
                            self.line(format!("{} {} = {};", __h0, t, g));
                            self.reg_temp(&t, et.clone());
                            t
                        } else {
                            g
                        }
                    }
                    Ty::Dict(k, v) => {
                        let g = format!(
                            "still_dict_{}_{}_get({}, {}, __FILE__, __LINE__)",
                            ty_tag(&resolve_deep(k)),
                            ty_tag(&resolve_deep(v)),
                            oe,
                            ie
                        );
                        self.set_line(&sp);
                        let vt = resolve_deep(v);
                        if self.ty_has_refs(&vt) || matches!(vt, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                            let t = self.newtmp();
                            let __h0 = self.ctype(&vt);
                            self.line(format!("{} {} = {};", __h0, t, g));
                            self.reg_temp(&t, vt.clone());
                            t
                        } else {
                            g
                        }
                    }
                    Ty::Str => {
                        let t = self.newtmp();
                        self.set_line(&sp);
                        self.line(format!("still_str* {} = still_str_index({}, {}, __FILE__, __LINE__);", t, oe, ie));
                        self.reg_temp(&t, Ty::Str);
                        t
                    }
                    Ty::Tuple(_ts) => match &idx.x {
                        X::Int(k) => format!("{}.f{}", oe, k),
                        _ => format!("{}", oe),
                    },
                    Ty::Named(ref q) => {
                        if self.prog.structs.get(q).map(|si| si.is_soa).unwrap_or(false) {
                            if let X::Field(ref inner, ref fname) = obj.x {
                                let base = self.emit_expr(inner);
                                return format!(
                                    "{}->f_{}[still_soa_index({}, {}->len, __FILE__, __LINE__)]",
                                    base, fname, ie, base
                                );
                            }
                        }
                        format!("{}[{}]", oe, ie)
                    }
                    _ => format!("{}[{}]", oe, ie),
                }
            }
            X::Slice(obj, a, b, s) => {
                let oe = self.as_var(obj);
                let oty = resolve_deep(&obj.ty);
                match (&oty, s) {
                    (Ty::List(e), None) => {
                        let tag = ty_tag(&resolve_deep(e));
                        let av = a.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "0".into());
                        let bv = b.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| format!("{}->len", oe));
                        let t = self.newtmp();
                        self.set_line(&sp);
                        self.line(format!("still_list_{}* {} = still_list_{}_slice({}, {}, {});", tag, t, tag, oe, av, bv));
                        self.reg_temp(&t, oty.clone());
                        t
                    }
                    (Ty::List(e), Some(step)) => {
                        let tag = ty_tag(&resolve_deep(e));
                        let av = a.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "STILL_SLICE_NONE".into());
                        let bv = b.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "STILL_SLICE_NONE".into());
                        let sv = self.emit_expr(step);
                        let t = self.newtmp();
                        self.set_line(&sp);
                        self.line(format!("still_list_{}* {} = still_list_{}_slice3({}, {}, {}, {});", tag, t, tag, oe, av, bv, sv));
                        self.reg_temp(&t, oty.clone());
                        t
                    }
                    _ => {
                        let t = self.newtmp();
                        self.set_line(&sp);
                        match s {
                            None => {
                                let av = a.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "0".into());
                                let bv = b.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| format!("{}->len", oe));
                                self.line(format!("still_str* {} = still_str_slice({}, {}, {});", t, oe, av, bv));
                            }
                            Some(step) => {
                                let sv = self.emit_expr(step);
                                let av = a.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "STILL_SLICE_NONE".into());
                                let bv = b.as_ref().map(|x| self.emit_expr(x)).unwrap_or_else(|| "STILL_SLICE_NONE".into());
                                self.line(format!("still_str* {} = still_str_slice3({}, {}, {}, {});", t, oe, av, bv, sv));
                            }
                        }
                        self.reg_temp(&t, Ty::Str);
                        t
                    }
                }
            }
            X::Field(obj, name) => {
                let oe = self.emit_expr(obj);
                if let Ty::Named(ref q) = resolve_deep(&obj.ty) {
                    if self.prog.structs.get(q).map(|si| si.is_soa).unwrap_or(false) {
                        return format!("{}->f_{}", oe, name);
                    }
                }
                format!("{}.{}", oe, name)
            }
            X::Unwrap(v) => {
                let e = self.emit_expr(v);
                let vty = resolve_deep(&v.ty);
                match &vty {
                    Ty::Nullable(inner) => {
                        let i = resolve_deep(inner);
                        match i {
                            Ty::Str | Ty::List(_) | Ty::Dict(..) => e,
                            _ => format!("{}.v", e),
                        }
                    }
                    _ => e,
                }
            }
            X::AsFloat(v) => {
                let e = self.emit_expr(v);
                format!("(double)({})", e)
            }
            X::AsStr(v) => self.emit_as_str(v),
            X::Method(recv, name, args) => self.emit_method(recv, name, args, &sp),
            X::Builtin(kind, args) => self.emit_builtin(*kind, args, &sp, tx),
        }
    }

    fn rel_if_owned(&mut self, expr: &str, ty: &Ty) {
        if self.is_temp(expr) {
            self.unreg_temp(expr);
            let d = self.drop_stmt(&resolve_deep(ty), expr);
            if !d.is_empty() {
                self.line(d);
            }
        }
    }

    fn emit_cond_value(&mut self, le: &str, lty: &Ty, re: &str, rty: &Ty, resty: &Ty, is_and: bool) -> String {
        let lt = self.truthy(lty, le);
        let tvar = self.newtmp();
        let __h0 = self.ctype(&resolve_deep(resty));
        self.line(format!("{} {};", __h0, tvar));
        if is_and {
            self.line(format!("if ({}) {{", lt));
            let rv = self.owned_value(re, rty);
            self.line(format!("{} = {};", tvar, rv));
            self.rel_if_owned(le, lty);
            self.raw("    } else {");
            let lv = self.owned_value(le, lty);
            self.line(format!("{} = {};", tvar, lv));
            self.raw("    }");
        } else {
            self.line(format!("if ({}) {{", lt));
            let lv = self.owned_value(le, lty);
            self.line(format!("{} = {};", tvar, lv));
            self.rel_if_owned(re, rty);
            self.raw("    } else {");
            let rv = self.owned_value(re, rty);
            self.line(format!("{} = {};", tvar, rv));
            self.raw("    }");
        }
        if self.ty_has_refs(&resolve_deep(resty)) || matches!(resolve_deep(resty), Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
            self.reg_temp(&tvar, resolve_deep(resty));
        }
        tvar
    }

    fn emit_cmp(&mut self, op: CmpOp, le: &str, lty: &Ty, re: &str, rty: &Ty, sp: &Span) -> String {
        self.set_line(sp);
        let lt = resolve_deep(lty);
        let rt = resolve_deep(rty);
        let o = Emitter::cmp_op_str(op);
        match (&lt, &rt) {
            (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float) | (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => {
                format!("({} {} {})", le, o, re)
            }
            (Ty::Str, Ty::Str) => {
                if matches!(op, CmpOp::Eq | CmpOp::Ne) {
                    format!("still_str_cmp({}, {}) {} 0", le, re, o)
                } else {
                    format!("still_str_cmp({}, {}) {} 0", le, re, o)
                }
            }
            (Ty::Bool, Ty::Bool) => format!("({} {} {})", le, o, re),
            _ => {
                let eqf = self.eq_fn(&lt);
                if matches!(op, CmpOp::Eq) {
                    format!("{}({}, {})", eqf, le, re)
                } else {
                    format!("(!{}({}, {}))", eqf, le, re)
                }
            }
        }
    }

    fn tostr_fn(&mut self, ty: &Ty) -> String {
        let t = resolve_deep(ty);
        match &t {
            Ty::Int | Ty::Never => "still_sb_add_int".into(),
            Ty::Float => "still_sb_add_float".into(),
            Ty::Bool => "still_sb_add_bool".into(),
            Ty::Str => "still_sb_add_str".into(),
            Ty::NoneTy => "still_sb_add_none".into(),
            Ty::Var(_) => "still_sb_add_int".into(),
            Ty::List(e) => {
                let et = resolve_deep(e);
                let tag = ty_tag(&et);
                self.need_list(&et);
                let key = format!("tolist_{}", tag);
                if self.helper_keys.insert(key.clone()) {
                    let inner = self.tostr_fn(&et);
                    let code = format!(
                        "static void still_tostr_add_{tag}(still_sb* sb, still_list_{tag}* l) {{\n    still_sb_c(sb, '[');\n    for (int64_t _i = 0; _i < l->len; _i++) {{\n        if (_i) still_sb_raw(sb, \", \");\n        {inner}(sb, l->data[_i]);\n    }}\n    still_sb_c(sb, ']');\n}}\n",
                        tag = tag,
                        inner = inner
                    );
                    self.helpers.push(code);
                }
                format!("still_tostr_add_{}", tag)
            }
            Ty::Dict(k, v) => {
                let kt = resolve_deep(k);
                let vt = resolve_deep(v);
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                let kadd = self.tostr_fn(&kt);
                let vadd = self.tostr_fn(&vt);
                let key = format!("todict_{}_{}", ktag, vtag);
                if self.helper_keys.insert(key.clone()) {
                    let code = format!(
                        "static void still_tostr_add_{ktag}_{vtag}(still_sb* sb, still_dict_{ktag}_{vtag}* d) {{\n    still_sb_c(sb, '{{');\n    bool _first = true;\n    for (int64_t _i = 0; _i < d->cap; _i++) {{\n        if (d->slots[_i].state != 1) continue;\n        if (!_first) still_sb_raw(sb, \", \");\n        _first = false;\n        {kadd}(sb, d->slots[_i].key);\n        still_sb_raw(sb, \": \");\n        {vadd}(sb, d->slots[_i].val);\n    }}\n    still_sb_c(sb, '}}');\n}}\n",
                        ktag = ktag,
                        vtag = vtag,
                        kadd = kadd,
                        vadd = vadd
                    );
                    self.helpers.push(code);
                }
                format!("still_tostr_add_{}_{}", ktag, vtag)
            }
            Ty::Tuple(ts) => {
                let ttag = self.tup_tag(ts);
                self.need_tuple(ts);
                let key = format!("totup_{}", ttag);
                if self.helper_keys.insert(key.clone()) {
                    let mut body = String::new();
                    for (i, fty) in ts.iter().enumerate() {
                        let fadd = self.tostr_fn(&resolve_deep(fty));
                        if i > 0 {
                            body.push_str("    still_sb_raw(sb, \", \");\n");
                        }
                        body.push_str(&format!("    {fadd}(sb, _p.f{i});\n", fadd = fadd, i = i));
                    }
                    let code = format!(
                        "static void still_tostr_add_tup_{t}(still_sb* sb, still_tup_{t} _p) {{\n    still_sb_c(sb, '(');\n{body}    still_sb_c(sb, ')');\n}}\n",
                        t = ttag,
                        body = body
                    );
                    self.helpers.push(code);
                }
                format!("still_tostr_add_tup_{}", ttag)
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return "still_tostr_add_soa_stub".into();
                    }
                    let cname = sanitize(q);
                    let key = format!("tost_{}", cname);
                    if self.helper_keys.insert(key.clone()) {
                        let mut body = String::new();
                        for (i, (fname, fty)) in si.fields.iter().enumerate() {
                            let fadd = self.tostr_fn(&resolve_deep(fty));
                            if i > 0 {
                                body.push_str("    still_sb_raw(sb, \", \");\n");
                            }
                            body.push_str(&format!("    {fadd}(sb, _p.{f});\n", fadd = fadd, f = fname));
                        }
                        let code = format!(
                            "static void still_tostr_add_st_{c}(still_sb* sb, still_st_{c} _p) {{\n    still_sb_raw(sb, \"{n}(\");\n{body}    still_sb_c(sb, ')');\n}}\n",
                            c = cname,
                            n = si.name,
                            body = body
                        );
                        self.helpers.push(code);
                    }
                    format!("still_tostr_add_st_{}", cname)
                } else {
                    "still_sb_add_none".into()
                }
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(inner);
                let inner_add = self.tostr_fn(&i);
                let tag = ty_tag(&i);
                let is_ref = matches!(i, Ty::Str | Ty::List(_) | Ty::Dict(..))
                    || matches!(&i, Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false));
                let key = format!("toopt_{}", tag);
                if self.helper_keys.insert(key.clone()) {
                    let code = if is_ref {
                        format!(
                            "static void still_tostr_add_opt_{tag}(still_sb* sb, void* p) {{ if (p) {{ {inner}(sb, p); }} else {{ still_sb_raw(sb, \"none\"); }} }}\n",
                            tag = tag,
                            inner = inner_add
                        )
                    } else {
                        self.need_opt(&i);
                        format!(
                            "static void still_tostr_add_opt_{tag}(still_sb* sb, still_opt_{tag} _p) {{ if (_p.has) {{ {inner}(sb, _p.v); }} else {{ still_sb_raw(sb, \"none\"); }} }}\n",
                            tag = tag,
                            inner = inner_add
                        )
                    };
                    self.helpers.push(code);
                }
                format!("still_tostr_add_opt_{}", tag)
            }
        }
    }

    fn emit_as_str(&mut self, v: &Tx) -> String {
        let e = self.emit_expr(v);
        let t = self.newtmp();
        self.set_line(&v.span);
        match resolve_deep(&v.ty) {
            Ty::Int => {
                self.line(format!("still_str* {} = still_str_from_int({});", t, e));
                self.reg_temp(&t, Ty::Str);
            }
            Ty::Float => {
                self.line(format!("still_str* {} = still_str_from_float({});", t, e));
                self.reg_temp(&t, Ty::Str);
            }
            Ty::Bool => {
                self.line(format!("still_str* {} = still_str_from_bool({});", t, e));
                self.reg_temp(&t, Ty::Str);
            }
            Ty::Str => {
                let owned = self.owned_value(&e, &Ty::Str);
                self.line(format!("still_str* {} = {};", t, owned));
                self.reg_temp(&t, Ty::Str);
            }
            Ty::List(_) | Ty::Dict(..) | Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_) => {
                let add = self.tostr_fn(&resolve_deep(&v.ty));
                let sb = self.newtmp();
                self.line(format!("still_sb* {} = still_sb_new();", sb));
                self.line(format!("{}({}, {});", add, sb, e));
                self.line(format!("still_str* {} = still_sb_finish({});", t, sb));
                self.reg_temp(&t, Ty::Str);
            }
            _ => {
                self.line(format!("still_str* {} = still_str_new(\"none\", 4);", t));
                self.reg_temp(&t, Ty::Str);
            }
        }
        t
    }

    fn emit_method(&mut self, recv: &Tx, name: &str, args: &[Tx], sp: &Span) -> String {
        let re = self.emit_expr(recv);
        let rty = resolve_deep(&recv.ty);
        self.set_line(sp);
        match &rty {
            Ty::List(e) => {
                let et = resolve_deep(e);
                let tag = ty_tag(&et);
                self.need_list(&et);
                match name {
                    "push" => {
                        let v = self.emit_expr(&args[0]);
                        let av = self.adapt_owned(&v, &resolve_deep(&args[0].ty), &et);
                        format!("still_list_{}_push({}, {})", tag, re, av)
                    }
                    "pop" => {
                        let t = self.newtmp();
                        let __h0 = self.ctype(&et);
                        self.line(format!("{} {} = still_list_{}_pop({}, __FILE__, __LINE__);", __h0, t, tag, re));
                        if self.ty_has_refs(&et) || matches!(et, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                            self.reg_temp(&t, et.clone());
                        }
                        t
                    }
                    "len" => format!("{}->len", re),
                    "clear" => format!("still_list_{}_clear({})", tag, re),
                    _ => "0".into(),
                }
            }
            Ty::Str => match name {
                "len" => format!("{}->len", re),
                "upper" | "lower" | "trim" => {
                    let t = self.newtmp();
                    self.line(format!("still_str* {} = still_str_{}({});", t, name, re));
                    self.reg_temp(&t, Ty::Str);
                    t
                }
                "contains" | "starts" | "ends" => {
                    let a = self.emit_expr(&args[0]);
                    format!("still_str_{}({}, {})", name, re, a)
                }
                "split" => {
                    let a = self.emit_expr(&args[0]);
                    let t = self.newtmp();
                    self.line(format!("still_list_s* {} = still_str_split({}, {});", t, re, a));
                    self.reg_temp(&t, Ty::List(Rc::new(Ty::Str)));
                    t
                }
                "replace" => {
                    let a = self.emit_expr(&args[0]);
                    let b = self.emit_expr(&args[1]);
                    let t = self.newtmp();
                    self.line(format!("still_str* {} = still_str_replace({}, {}, {});", t, re, a, b));
                    self.reg_temp(&t, Ty::Str);
                    t
                }
                "join" => {
                    let a = self.emit_expr(&args[0]);
                    let t = self.newtmp();
                    self.line(format!("still_str* {} = still_str_join({}, {});", t, re, a));
                    self.reg_temp(&t, Ty::Str);
                    t
                }
                _ => "0".into(),
            },
            Ty::Dict(k, v) => {
                let ktag = ty_tag(&resolve_deep(k));
                let vtag = ty_tag(&resolve_deep(v));
                match name {
                    "len" => format!("{}->len", re),
                    "clear" => format!("still_dict_{}_{}_clear({})", ktag, vtag, re),
                    "get" => {
                        // d.get(key, default): value ownership moves in; key is borrowed
                        // (the statement-temp release of the key matches plain dict reads).
                        let vty = resolve_deep(v);
                        let kt = self.emit_expr(&args[0]);
                        let dv = self.emit_expr(&args[1]);
                        let t = self.newtmp();
                        let ct = self.ctype(&vty);
                        self.set_line(&sp);
                        self.line(format!("{} {};", ct, t));
                        self.line(format!("if (still_dict_{}_{}_has({}, {})) {{", ktag, vtag, re, kt));
                        self.line(format!("{} = still_dict_{}_{}_get({}, {}, __FILE__, __LINE__);", t, ktag, vtag, re, kt));
                        self.line("} else {".into());
                        self.unreg_temp(&dv);
                        self.line(format!("{} = {};", t, dv));
                        self.line("}".into());
                        self.reg_temp(&t, vty);
                        t
                    }
                    "pop" => {
                        let vty = resolve_deep(v);
                        let kt = self.emit_expr(&args[0]);
                        let t = self.newtmp();
                        let ct = self.ctype(&vty);
                        self.set_line(&sp);
                        self.line(format!("{} {} = still_dict_{}_{}_pop({}, {}, __FILE__, __LINE__);", ct, t, ktag, vtag, re, kt));
                        self.reg_temp(&t, vty);
                        t
                    }
                    _ => "0".into(),
                }
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        match name {
                            "soa_push" => {
                                let mut vals = Vec::new();
                                for a in args {
                                    let e = self.emit_expr(a);
                                    let aty = resolve_deep(&a.ty);
                                    vals.push(self.adapt_owned(&e, &aty, &aty));
                                }
                                format!("still_soa_{}_push({}, {})", si.name, re, vals.join(", "))
                            }
                            "soa_clear" => format!("still_soa_{}_clear({})", si.name, re),
                            "soa_len" => format!("{}->len", re),
                            _ => "0".into(),
                        }
                    } else {
                        "0".into()
                    }
                } else {
                    "0".into()
                }
            }
            _ => "0".into(),
        }
    }
}

fn _dummy_v(t: &Ty) -> Ty {
    match t {
        Ty::Dict(_, v) => (**v).clone(),
        _ => Ty::Int,
    }
}

impl Emitter {
    fn print_call(&mut self, a: &Tx) {
        let e = self.emit_expr(a);
        let t = resolve_deep(&a.ty);
        let f = self.print_fn(&t);
        self.line(format!("{}({});", f, e));
    }

    fn repr_fn(&mut self, ty: &Ty) -> String {
        let t = resolve_deep(ty);
        match &t {
            Ty::Str => {
                if self.helper_keys.insert("pstrq".to_string()) {
                    self.helpers
                        .push("static void still_print_strq(still_str* s) { fputc('\"', stdout); still_print_str(s); fputc('\"', stdout); }\n".to_string());
                }
                "still_print_strq".into()
            }
            Ty::List(e) => {
                let et = resolve_deep(e);
                let tag = ty_tag(&et);
                self.need_list(&et);
                let inner = self.repr_fn(&et);
                let key = format!("plist_{}", tag);
                if self.helper_keys.insert(key.clone()) {
                    let code = format!(
                        "static void still_print_list_{tag}(still_list_{tag}* l) {{\n    fputc('[', stdout);\n    for (int64_t _i = 0; _i < l->len; _i++) {{\n        if (_i) fputc(',', stdout), fputc(' ', stdout);\n        {inner}(l->data[_i]);\n    }}\n    fputc(']', stdout);\n}}\n",
                        tag = tag,
                        inner = inner
                    );
                    self.helpers.push(code);
                }
                format!("still_print_list_{}", tag)
            }
            Ty::Dict(k, v) => {
                let kt = resolve_deep(k);
                let vt = resolve_deep(v);
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                let kpr = self.repr_fn(&kt);
                let vpr = self.repr_fn(&vt);
                let key = format!("pdict_{}_{}", ktag, vtag);
                if self.helper_keys.insert(key.clone()) {
                    let code = format!(
                        "static void still_print_dict_{ktag}_{vtag}(still_dict_{ktag}_{vtag}* d) {{\n    fputc('{{', stdout);\n    bool _first = true;\n    for (int64_t _i = 0; _i < d->cap; _i++) {{\n        if (d->slots[_i].state != 1) continue;\n        if (!_first) fputc(',', stdout), fputc(' ', stdout);\n        _first = false;\n        {kpr}(d->slots[_i].key);\n        fputc(':', stdout), fputc(' ', stdout);\n        {vpr}(d->slots[_i].val);\n    }}\n    fputc('}}', stdout);\n}}\n",
                        ktag = ktag,
                        vtag = vtag,
                        kpr = kpr,
                        vpr = vpr
                    );
                    self.helpers.push(code);
                }
                format!("still_print_dict_{}_{}", ktag, vtag)
            }
            Ty::Tuple(ts) => {
                let ttag = self.tup_tag(ts);
                self.need_tuple(ts);
                let key = format!("ptup_{}", ttag);
                if self.helper_keys.insert(key.clone()) {
                    let mut body = String::new();
                    for (i, fty) in ts.iter().enumerate() {
                        let fpr = self.repr_fn(&resolve_deep(fty));
                        if i > 0 {
                            body.push_str("        fputc(',', stdout), fputc(' ', stdout);\n");
                        }
                        body.push_str(&format!("        {}(_p.f{});\n", fpr, i));
                    }
                    let code = format!(
                        "static void still_print_tup_{t}(still_tup_{t} _p) {{\n    fputc('(', stdout);\n{body}    fputc(')', stdout);\n}}\n",
                        t = ttag,
                        body = body
                    );
                    self.helpers.push(code);
                }
                format!("still_print_tup_{}", ttag)
            }
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return format!("still_print_soa_{}", si.name);
                    }
                    let cname = sanitize(q);
                    let key = format!("pst_{}", cname);
                    if self.helper_keys.insert(key.clone()) {
                        let mut body = String::new();
                        for (i, (fname, fty)) in si.fields.iter().enumerate() {
                            let fpr = self.repr_fn(&resolve_deep(fty));
                            if i > 0 {
                                body.push_str("    fputc(',', stdout), fputc(' ', stdout);\n");
                            }
                            body.push_str(&format!("    {}(_p.{});\n", fpr, fname));
                        }
                        let code = format!(
                            "static void still_print_st_{c}(still_st_{c} _p) {{\n    fputs(\"{n}(\", stdout);\n{body}    fputc(')', stdout);\n}}\n",
                            c = cname,
                            n = si.name,
                            body = body
                        );
                        self.helpers.push(code);
                    }
                    format!("still_print_st_{}", cname)
                } else {
                    "still_print_none".into()
                }
            }
            Ty::Nullable(inner) => {
                let i = resolve_deep(inner);
                match i {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) => {
                        let inner_print = self.repr_fn(&i);
                        let tag = ty_tag(&i);
                        let key = format!("poptref_{}", tag);
                        if self.helper_keys.insert(key.clone()) {
                            let code = format!(
                                "static void still_print_optref_{tag}(void* p) {{ if (p) {{ {inner}(p); }} else {{ still_print_none(); }} }}\n",
                                tag = tag,
                                inner = inner_print
                            );
                            self.helpers.push(code);
                        }
                        format!("still_print_optref_{}", tag)
                    }
                    Ty::Named(ref q) if self.prog.structs.get(q).map(|s| s.is_soa).unwrap_or(false) => {
                        let inner_print = self.repr_fn(&i);
                        let tag = ty_tag(&i);
                        let key = format!("poptref_{}", tag);
                        if self.helper_keys.insert(key.clone()) {
                            let code = format!(
                                "static void still_print_optref_{tag}(void* p) {{ if (p) {{ {inner}(p); }} else {{ still_print_none(); }} }}\n",
                                tag = tag,
                                inner = inner_print
                            );
                            self.helpers.push(code);
                        }
                        format!("still_print_optref_{}", tag)
                    }
                    _ => {
                        let tag = ty_tag(&i);
                        self.need_opt(&i);
                        let inner_print = self.repr_fn(&i);
                        let key = format!("popt_{}", tag);
                        if self.helper_keys.insert(key.clone()) {
                            let code = format!(
                                "static void still_print_opt_{tag}(still_opt_{tag} _p) {{ if (_p.has) {{ {inner}(_p.v); }} else {{ still_print_none(); }} }}\n",
                                tag = tag,
                                inner = inner_print
                            );
                            self.helpers.push(code);
                        }
                        format!("still_print_opt_{}", tag)
                    }
                }
            }
            _ => self.print_fn(&t),
        }
    }

    fn print_fn(&mut self, ty: &Ty) -> String {
        let t = resolve_deep(ty);
        match &t {
            Ty::Int | Ty::Never => "still_print_int".into(),
            Ty::Float => "still_print_float".into(),
            Ty::Bool => "still_print_bool".into(),
            Ty::Str => "still_print_str".into(),
            Ty::NoneTy => "still_print_none".into(),
            Ty::List(_) | Ty::Dict(..) | Ty::Tuple(_) | Ty::Named(_) => self.repr_fn(&t),
            Ty::Nullable(inner) => {
                let i = resolve_deep(inner);
                if matches!(i, Ty::Str) {
                    let key = "poptref_top".to_string();
                    if self.helper_keys.insert(key) {
                        self.helpers
                            .push("static void still_print_optref_top(void* p) { if (p) { still_print_str(p); } else { still_print_none(); } }\n".to_string());
                    }
                    "still_print_optref_top".into()
                } else {
                    self.repr_fn(&t)
                }
            }
            Ty::Var(_) => "still_print_int".into(),
        }
    }

    fn emit_builtin(&mut self, kind: Bi, args: &[Tx], sp: &Span, tx: &Tx) -> String {
        self.set_line(sp);
        match kind {
            Bi::Print => {
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.line("still_print_sep();".into());
                    }
                    if matches!(resolve(&a.ty), Ty::Never) {
                        self.emit_expr(a);
                        continue;
                    }
                    self.print_call(a);
                }
                self.line("still_print_nl();".into());
                "0".into()
            }
            Bi::Len => {
                let e = self.emit_expr(&args[0]);
                format!("{}->len", e)
            }
            Bi::Push => {
                let l = self.emit_expr(&args[0]);
                let v = self.emit_expr(&args[1]);
                let lty = resolve_deep(&args[0].ty);
                let et = match &lty {
                    Ty::List(e) => resolve_deep(e),
                    _ => Ty::Int,
                };
                let tag = ty_tag(&et);
                self.need_list(&et);
                let av = self.adapt_owned(&v, &resolve_deep(&args[1].ty), &et);
                format!("still_list_{}_push({}, {})", tag, l, av)
            }
            Bi::Pop => {
                let l = self.emit_expr(&args[0]);
                let lty = resolve_deep(&args[0].ty);
                let et = match &lty {
                    Ty::List(e) => resolve_deep(e),
                    _ => Ty::Int,
                };
                let tag = ty_tag(&et);
                self.need_list(&et);
                let t = self.newtmp();
                let __h0 = self.ctype(&et);
                self.line(format!("{} {} = still_list_{}_pop({}, __FILE__, __LINE__);", __h0, t, tag, l));
                if self.ty_has_refs(&et) || matches!(et, Ty::Tuple(_) | Ty::Named(_) | Ty::Nullable(_)) {
                    self.reg_temp(&t, et.clone());
                }
                t
            }
            Bi::Keys => {
                let d = self.emit_expr(&args[0]);
                let (kt, vt) = match resolve_deep(&args[0].ty) {
                    Ty::Dict(k, v) => (resolve_deep(&k), resolve_deep(&v)),
                    _ => (Ty::Int, Ty::Int),
                };
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                let t = self.newtmp();
                self.line(format!("still_list_{}* {} = still_dict_{}_{}_keys({});", ktag, t, ktag, vtag, d));
                self.reg_temp(&t, Ty::List(Rc::new(kt)));
                t
            }
            Bi::Pairs => {
                let d = self.emit_expr(&args[0]);
                let (kt, vt) = match resolve_deep(&args[0].ty) {
                    Ty::Dict(k, v) => (resolve_deep(&k), resolve_deep(&v)),
                    _ => (Ty::Int, Ty::Int),
                };
                let ktag = ty_tag(&kt);
                let vtag = ty_tag(&vt);
                let t = self.newtmp();
                let ltag = ty_tag(&Ty::Tuple(Rc::new(vec![kt.clone(), vt.clone()])));
                self.line(format!("still_list_{}* {} = still_dict_{}_{}_pairs({});", ltag, t, ktag, vtag, d));
                self.reg_temp(&t, Ty::List(Rc::new(Ty::Tuple(Rc::new(vec![kt, vt])))));
                t
            }
            Bi::StrOf => self.emit_as_str(&args[0]),
            Bi::IntOf => {
                let e = self.emit_expr(&args[0]);
                match resolve_deep(&args[0].ty) {
                    Ty::Int => e,
                    Ty::Float => format!("(int64_t)({})", e),
                    Ty::Str => format!("still_str_to_int({}, __FILE__, __LINE__)", e),
                    Ty::Bool => format!("(int64_t)({})", e),
                    _ => "0".into(),
                }
            }
            Bi::FloatOf => {
                let e = self.emit_expr(&args[0]);
                match resolve_deep(&args[0].ty) {
                    Ty::Float => e,
                    Ty::Int => format!("(double)({})", e),
                    Ty::Str => format!("still_str_to_float({}, __FILE__, __LINE__)", e),
                    Ty::Bool => format!("(double)({})", e),
                    _ => "0.0".into(),
                }
            }
            Bi::BoolOf => {
                let e = self.emit_expr(&args[0]);
                self.truthy(&resolve_deep(&args[0].ty), &e)
            }
            Bi::Abs => {
                let e = self.emit_expr(&args[0]);
                match resolve_deep(&args[0].ty) {
                    Ty::Int => format!("still_iabs({})", e),
                    _ => format!("fabs({})", e),
                }
            }
            Bi::Min | Bi::Max => {
                let pick_lt = kind == Bi::Min;
                if args.len() == 1 {
                    let l = self.emit_expr(&args[0]);
                    let lty = resolve_deep(&args[0].ty);
                    let et = match &lty {
                        Ty::List(e) => resolve_deep(e),
                        _ => Ty::Int,
                    };
                    let t = self.newtmp();
                    self.line(format!(
                        "if ({}->len == 0) still_panic(__FILE__, __LINE__, \"{} of an empty list\");",
                        l,
                        if pick_lt { "min" } else { "max" }
                    ));
                    let init = self.retain_expr(&et).replace("_v", &format!("{}->data[0]", l));
                    let __h0 = self.ctype(&et);
                    self.line(format!("{} {} = {};", __h0, t, init));
                    self.line(format!("for (int64_t __i = 1; __i < {}->len; __i++) {{", l));
                    let cmp = if matches!(resolve(&et), Ty::Str) {
                        if pick_lt {
                            format!("still_str_cmp({}->data[__i], {}) < 0", l, t)
                        } else {
                            format!("still_str_cmp({}->data[__i], {}) > 0", l, t)
                        }
                    } else if pick_lt {
                        format!("{}->data[__i] < {}", l, t)
                    } else {
                        format!("{}->data[__i] > {}", l, t)
                    };
                    self.line(format!("if ({}) {{", cmp));
                    let rel = self.release_stmt_expr(&et, &t);
                    if !rel.is_empty() {
                        self.line(rel);
                    }
                    let ni = self.retain_expr(&et).replace("_v", &format!("{}->data[__i]", l));
                    self.line(format!("{} = {};", t, ni));
                    self.raw("    }");
                    self.raw("    }");
                    if self.ty_has_refs(&et) || matches!(et, Ty::Tuple(_) | Ty::Named(_)) {
                        self.reg_temp(&t, et.clone());
                    }
                    t
                } else {
                    let rty = resolve_deep(&tx.ty);
                    let t = self.newtmp();
                    let first = self.emit_expr(&args[0]);
                    let owned = self.owned_value(&first, &rty);
                    let __h0 = self.ctype(&rty);
                    self.line(format!("{} {} = {};", __h0, t, owned));
                    for a in &args[1..] {
                        let e = self.emit_expr(a);
                        let cmp = if matches!(resolve(&rty), Ty::Str) {
                            if pick_lt {
                                format!("still_str_cmp({}, {}) < 0", e, t)
                            } else {
                                format!("still_str_cmp({}, {}) > 0", e, t)
                            }
                        } else if pick_lt {
                            format!("{} < {}", e, t)
                        } else {
                            format!("{} > {}", e, t)
                        };
                        self.line(format!("if ({}) {{", cmp));
                        let rel = self.release_stmt_expr(&rty, &t);
                        if !rel.is_empty() {
                            self.line(rel);
                        }
                        let nv = self.owned_value(&e, &rty);
                        self.line(format!("{} = {};", t, nv));
                        self.raw("    }");
                    }
                    if self.ty_has_refs(&rty) || matches!(rty, Ty::Tuple(_) | Ty::Named(_)) {
                        self.reg_temp(&t, rty.clone());
                    }
                    t
                }
            }
            Bi::Sum => {
                let l = self.emit_expr(&args[0]);
                let lty = resolve_deep(&args[0].ty);
                let et = match &lty {
                    Ty::List(e) => resolve_deep(e),
                    _ => Ty::Int,
                };
                let t = self.newtmp();
                if matches!(resolve(&et), Ty::Float) {
                    self.line(format!("double {} = 0;", t));
                    self.line(format!("for (int64_t __i = 0; __i < {}->len; __i++) {{ {} += {}->data[__i]; }}", l, t, l));
                } else {
                    self.line(format!("int64_t {} = 0;", t));
                    self.line(format!(
                        "for (int64_t __i = 0; __i < {}->len; __i++) {{ {} = still_add({}, {}->data[__i], __FILE__, __LINE__); }}",
                        l, t, t, l
                    ));
                }
                t
            }
            Bi::Range | Bi::Range2 | Bi::Range3 => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.emit_expr(a));
                }
                let t = self.newtmp();
                let f = match kind {
                    Bi::Range => format!("still_range({})", vals[0]),
                    Bi::Range2 => format!("still_range2({}, {})", vals[0], vals[1]),
                    _ => format!("still_range3({}, {}, {})", vals[0], vals[1], vals[2]),
                };
                self.line(format!("still_list_i* {} = {};", t, f));
                self.reg_temp(&t, Ty::List(Rc::new(Ty::Int)));
                t
            }
            Bi::Enumerate => {
                let l = self.emit_expr(&args[0]);
                let et = match resolve_deep(&args[0].ty) {
                    Ty::List(e) => resolve_deep(&e),
                    _ => Ty::Int,
                };
                let tup = vec![Ty::Int, et.clone()];
                let ttag = self.tup_tag(&tup);
                self.need_tuple(&tup);
                let t = self.newtmp();
                self.line(format!("still_list_{}* {} = still_list_{}_alloc({}->len);", ttag, t, ttag, l));
                let retain = self.retain_expr(&et);
                let etc = self.ctype(&et);
                self.line(format!(
                    "for (int64_t __i = 0; __i < {}->len; __i++) {{ still_tup_{} _p; _p.f0 = __i; {{ {} _v = {}->data[__i]; (void)_v; _p.f1 = {}; }} still_list_{}_push({}, _p); }}",
                    l, ttag, etc, l, retain, ttag, t
                ));
                self.reg_temp(&t, Ty::List(Rc::new(Ty::Tuple(Rc::new(tup)))));
                t
            }
            Bi::Reversed => {
                let l = self.emit_expr(&args[0]);
                let et = match resolve_deep(&args[0].ty) {
                    Ty::List(e) => resolve_deep(&e),
                    _ => Ty::Int,
                };
                let tag = ty_tag(&et);
                let t = self.newtmp();
                self.line(format!("still_list_{}* {} = still_list_{}_reversed({});", tag, t, tag, l));
                self.reg_temp(&t, Ty::List(Rc::new(et)));
                t
            }
            Bi::TypeOf => {
                let name = match resolve_deep(&args[0].ty) {
                    Ty::Int => "int".to_string(),
                    Ty::Float => "float".to_string(),
                    Ty::Bool => "bool".to_string(),
                    Ty::Str => "str".to_string(),
                    Ty::NoneTy => "none".to_string(),
                    Ty::List(_) => "list".to_string(),
                    Ty::Dict(..) => "dict".to_string(),
                    Ty::Tuple(_) => "tuple".to_string(),
                    Ty::Nullable(_) => "optional".to_string(),
                    Ty::Named(ref q) => self
                        .prog
                        .structs
                        .get(q)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| "type".to_string()),
                    _ => "unknown".to_string(),
                };
                let e = self.emit_expr(&args[0]);
                if !e.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '>' || c == '-') || e.contains('(') {
                    self.line(format!("(void)({});", e));
                }
                let t = self.newtmp();
                let nlen = name.len();
                self.line(format!("still_str* {} = still_str_new(\"{}\", {});", t, name, nlen));
                self.reg_temp(&t, Ty::Str);
                t
            }
            Bi::Input | Bi::InputPrompt => {
                let p = if args.is_empty() {
                    "NULL".to_string()
                } else {
                    self.emit_expr(&args[0])
                };
                let t = self.newtmp();
                self.line(format!("still_str* {} = still_input({});", t, p));
                self.reg_temp(&t, Ty::Str);
                t
            }
            Bi::Clock => "still_clock()".into(),
            Bi::Args => "still_args()".into(),
            Bi::Rand => "still_rand()".into(),
            Bi::RandBound => {
                let e = self.emit_expr(&args[0]);
                format!("still_rand_bound({})", e)
            }
            Bi::Seed => {
                let e = self.emit_expr(&args[0]);
                format!("still_seed({})", e)
            }
            Bi::Check | Bi::CheckMsg => {
                let c = self.emit_expr(&args[0]);
                let t = self.truthy(&resolve_deep(&args[0].ty), &c);
                let msg = if args.len() > 1 {
                    let m = self.emit_expr(&args[1]);
                    format!("{}->bytes", m)
                } else {
                    "NULL".to_string()
                };
                self.line(format!("if (!({})) still_check_fail(__FILE__, __LINE__, {});", t, msg));
                "0".into()
            }
            Bi::Fail => {
                let m = self.emit_expr(&args[0]);
                self.line(format!("still_fail({}, __FILE__, __LINE__);", m));
                "0".into()
            }
            Bi::Exit => {
                let e = self.emit_expr(&args[0]);
                self.line(format!("still_exit({});", e));
                "0".into()
            }
            Bi::Ord => {
                let e = self.emit_expr(&args[0]);
                format!("still_ord({}, __FILE__, __LINE__)", e)
            }
            Bi::Chr => {
                let e = self.emit_expr(&args[0]);
                let t = self.newtmp();
                self.line(format!("still_str* {} = still_chr({}, __FILE__, __LINE__);", t, e));
                self.reg_temp(&t, Ty::Str);
                t
            }
            Bi::ReadFile => {
                let e = self.emit_expr(&args[0]);
                let t = self.newtmp();
                self.line(format!("still_str* {} = still_read_file({});", t, e));
                self.reg_temp(&t, Ty::Str);
                t
            }
            Bi::WriteFile => {
                let p = self.emit_expr(&args[0]);
                let d = self.emit_expr(&args[1]);
                format!("still_write_file({}, {})", p, d)
            }
            Bi::GetEnv => {
                let e = self.emit_expr(&args[0]);
                let t = self.newtmp();
                self.line(format!("still_str* {} = still_get_env({});", t, e));
                t
            }
            Bi::SleepMs => {
                let e = self.emit_expr(&args[0]);
                self.line(format!("still_sleep_ms({});", e));
                "0".into()
            }
        }
    }
}
