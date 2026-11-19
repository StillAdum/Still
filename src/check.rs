use crate::ast::*;
use crate::span::{did_you_mean, err, Diag, Span};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct ModSrc {
    pub name: String,
    pub stmts: Vec<Stmt>,
}

pub const BUILTINS: &[&str] = &[
    "print", "len", "push", "pop", "keys", "pairs", "str", "int", "float", "bool", "abs", "min", "max", "sum", "dict_key_at", "read_dir", "file_exists",
    "range", "enumerate", "reversed", "type_of", "input", "clock", "rand", "seed", "check", "fail", "exit",
    "ord", "chr", "read_file", "write_file", "get_env", "sleep_ms", "args",
];

type FKey = (String, String);

struct Ctx {
    module: String,
    scopes: Vec<HashMap<String, u32>>,
    locals: Vec<(String, Ty)>,
    refined: HashSet<String>,
    loop_vars: HashSet<String>,
    ret: VarCell,
    ret_annotated: bool,
    self_param: bool,
    loops: usize,
    in_fn: bool,
    depth: u32,
    global_names: HashSet<String>,
    /// Names bound by the current match statement's binding patterns. Fresh
    /// binder names declare arm-locals even at top level, where ordinary
    /// fresh assignments become module globals.
    pat_binders: HashSet<String>,
    /// Qual of the generic instantiation whose method body is being checked
    /// (e.g. "m::SlotMap<i>"); bare constructor calls to the owner type
    /// inside the body resolve to this.
    owner_inst: Option<String>,
    /// Type-parameter substitution in scope for generic method bodies:
    /// param names and the concrete types they stand for.
    ty_params: Vec<String>,
    ty_args: Vec<Ty>,
}

impl Ctx {
    fn new(module: String, in_fn: bool) -> Ctx {
        Ctx {
            module,
            scopes: vec![HashMap::new()],
            locals: Vec::new(),
            refined: HashSet::new(),
            loop_vars: HashSet::new(),
            ret: Rc::new(std::cell::RefCell::new(None)),
            ret_annotated: false,
            self_param: false,
            loops: 0,
            in_fn,
            depth: 0,
            global_names: HashSet::new(),
            pat_binders: HashSet::new(),
            owner_inst: None,
            ty_params: Vec::new(),
            ty_args: Vec::new(),
        }
    }

    fn lookup(&self, name: &str) -> Option<u32> {
        for s in self.scopes.iter().rev() {
            if let Some(id) = s.get(name) {
                return Some(*id);
            }
        }
        None
    }

    fn declare(&mut self, name: &str, ty: Ty) -> u32 {
        let id = self.locals.len() as u32;
        self.locals.push((name.to_string(), ty));
        self.scopes.last_mut().unwrap().insert(name.to_string(), id);
        id
    }
}

pub struct Checker {
    pub diags: Vec<Diag>,
    mods: HashMap<String, Vec<Stmt>>,
    funcs: HashMap<FKey, Rc<FuncDef>>,
    fn_order: Vec<FKey>,
    types: HashMap<String, TypeDef>,
    short_types: HashMap<String, HashMap<String, String>>,
    externs: HashMap<String, ExternFn>,
    aliases: HashMap<String, HashMap<String, String>>,
    imports: HashMap<String, HashMap<String, (String, String)>>,
    prog: Program,
    insts: HashMap<String, usize>,
    pcache: HashMap<FKey, (Vec<Option<Ty>>, Vec<Option<Tx>>)>,
    gmap: HashMap<(String, String), String>,
    init_order: Vec<String>,
    /// Instantiated generic types by (module, short name): "SlotMap<i>" -> qual.
    inst_by_name: HashMap<(String, String), String>,
    /// Generic-struct instantiation depth guard (nested type args).
    inst_depth: u32,
}

pub fn check_program(mods: Vec<ModSrc>, main_module: &str, debug: bool) -> (Program, Vec<Diag>) {
    let mut c = Checker {
        diags: Vec::new(),
        mods: HashMap::new(),
        funcs: HashMap::new(),
        fn_order: Vec::new(),
        types: HashMap::new(),
        short_types: HashMap::new(),
        externs: HashMap::new(),
        aliases: HashMap::new(),
        imports: HashMap::new(),
        prog: Program::default(),
        insts: HashMap::new(),
        pcache: HashMap::new(),
        gmap: HashMap::new(),
        init_order: Vec::new(),
        inst_by_name: HashMap::new(),
        inst_depth: 0,
    };
    c.prog.debug = debug;
    c.run(mods, main_module);
    (c.prog, c.diags)
}

pub use crate::ast::sanitize;

/// C field name of an enum payload slot: `v_{Variant}_{index}`. Kept
/// variant-prefixed so every payload slot is a unique struct field.
pub fn enum_payload_field(variant: &str, index: usize) -> String {
    format!("v_{}_{}", variant, index)
}

impl Checker {
    fn run(&mut self, mods: Vec<ModSrc>, main_module: &str) {
        for m in mods {
            self.mods.insert(m.name.clone(), m.stmts.clone());
        }
        let mod_names: Vec<String> = self.mods.keys().cloned().collect();
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            let mut fnames: HashSet<String> = HashSet::new();
            let mut tnames: HashSet<String> = HashSet::new();
            for s in &stmts {
                match s {
                    Stmt::FuncDef(f) => {
                        if !fnames.insert(f.name.clone()) {
                            self.diags.push(err(&f.span, format!("function '{}' is already defined in this module", f.name)));
                            continue;
                        }
                        self.funcs.insert((m.clone(), f.name.clone()), Rc::new(f.clone()));
                        self.fn_order.push((m.clone(), f.name.clone()));
                    }
                    Stmt::TypeDef(t) => {
                        if !tnames.insert(t.name.clone()) {
                            self.diags.push(err(&t.span, format!("type '{}' is already defined in this module", t.name)));
                            continue;
                        }
                        let qual = format!("{}::{}", m, t.name);
                        self.types.insert(qual.clone(), t.clone());
                        self.short_types.entry(m.clone()).or_default().insert(t.name.clone(), qual);
                    }
                    Stmt::Extern(fns) => {
                        for e in fns {
                            if self.externs.contains_key(&e.name) {
                                continue;
                            }
                            self.validate_extern(e);
                            self.externs.insert(e.name.clone(), e.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            for s in &stmts {
                if let Stmt::Use { target, alias, names, span } = s {
                    // path-form uses ("use \"dir/file.strc\"") are registered by
                    // the loader under the file stem, so resolve to the stem too
                    let mod_key = if target.ends_with(".strc") {
                        std::path::Path::new(target)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| target.clone())
                    } else {
                        target.clone()
                    };
                    if !self.mods.contains_key(&mod_key) {
                        self.diags.push(err(span, format!("unknown module '{}'", target)));
                        continue;
                    }
                    let alias = alias.clone().unwrap_or_else(|| mod_key.clone());
                    self.aliases.entry(m.clone()).or_default().insert(alias, mod_key.clone());
                    if let Some(list) = names {
                        for n in list {
                            let tmod = mod_key.clone();
                            if self.funcs.contains_key(&(tmod.clone(), n.clone()))
                                || self.externs.contains_key(n)
                                || self.gmap_contains(&mod_key, n)
                            {
                                self.imports.entry(m.clone()).or_default().insert(n.clone(), (tmod, n.clone()));
                            } else if self.short_types.get(&mod_key).map(|s| s.contains_key(n)).unwrap_or(false) {
                                self.imports.entry(m.clone()).or_default().insert(n.clone(), (mod_key.clone(), n.clone()));
                            } else {
                                self.diags.push(err(span, format!("module '{}' has no '{}'", target, n)));
                            }
                        }
                    }
                }
            }
        }
        self.collect_traits();
        self.collect_enums();
        self.collect_structs();
        self.collect_enums();
        self.collect_impls();
        if !self.diags.is_empty() {
            return;
        }
        self.order_modules(main_module);
        if !self.diags.is_empty() {
            return;
        }
        let order = self.init_order.clone();
        for m in order {
            if m == main_module {
                continue;
            }
            let cs = self.check_module_init(&m);
            self.prog.module_inits.push((m.clone(), cs));
        }
        let main_cs = self.check_module_init(main_module);
        self.prog.main_body = main_cs;
        if !self.diags.is_empty() {
            return;
        }
        let keys = self.fn_order.clone();
        for (m, f) in keys {
            let fd = self.funcs[&(m.clone(), f.clone())].clone();
            let (ptys, _) = self.param_info(&m, &f);
            let all_annot = fd.params.iter().enumerate().all(|(i, p)| p.name == "self" || ptys[i].is_some());
            if all_annot {
                let _ = self.instantiate(&m, &f, &[], &fd.span, true);
            }
        }
        self.convert_externs();
        self.finalize_types();
    }

    fn gmap_contains(&self, module: &str, name: &str) -> bool {
        self.mods.get(module).map(|stmts| {
            stmts.iter().any(|s| {
                matches!(s, Stmt::Assign { targets, op: None, .. } if targets.len() == 1 && matches!(&targets[0], Expr::Ident(n, _) if n == name))
            })
        }).unwrap_or(false)
    }

    fn validate_extern(&mut self, e: &ExternFn) {
        let ok = |a: &TypeAnnot| matches!(a, TypeAnnot::Int | TypeAnnot::Float | TypeAnnot::Bool | TypeAnnot::Str);
        for (_, a) in &e.params {
            if !ok(a) {
                self.diags.push(err(&e.span, format!("extern parameter types must be int, float, bool or str (use a wrapper for anything else)")));
            }
        }
        if let Some(r) = &e.ret {
            if !ok(r) {
                self.diags.push(err(&e.span, "extern return types must be int, float, bool or str (use a wrapper for anything else)".into()));
            }
        }
    }

    fn collect_structs(&mut self) {
        let quals: Vec<String> = self.types.keys().cloned().collect();
        for qual in &quals {
            let td = self.types[qual].clone();
            let module = qual.split("::").next().unwrap().to_string();
            if !td.params.is_empty() {
                // Generic type defs get no StructInfo until instantiated with
                // concrete type args (instantiate_struct); bare use is an
                // arity error.
                continue;
            }
            self.prog.structs.insert(
                qual.clone(),
                StructInfo {
                    module,
                    name: td.name.clone(),
                    fields: Vec::new(),
                    defaults: Vec::new(),
                    is_soa: td.is_soa(),
                    methods: Vec::new(),
                    statics: Vec::new(),
                    exported: td.attrs.iter().any(|a| a == "export"),
                    ty_args: Vec::new(),
                },
            );
        }
        for qual in &quals {
            let td = self.types[qual].clone();
            let module = qual.split("::").next().unwrap().to_string();
            if !td.params.is_empty() {
                continue; // generic: fields and methods resolve per instantiation
            }
            let mut fields: Vec<(String, Ty)> = Vec::new();
            let mut defaults: Vec<Option<Tx>> = Vec::new();
            for f in &td.fields {
                let ty = match self.annot_ty_in(&module, &f.annot, &f.span) {
                    Ok(t) => t,
                    Err(m) => {
                        self.diags.push(err(&f.span, m));
                        Ty::Int
                    }
                };
                if td.is_soa() && !matches!(resolve(&ty), Ty::Int | Ty::Float | Ty::Bool | Ty::Str) {
                    self.diags.push(err(&f.span, format!("@soa fields must be int, float, bool or str; field '{}' is {}", f.name, ty_name(&ty))));
                }
                if let Some(d) = &f.default {
                    match self.const_eval(d) {
                        Some(cv) if assign_ok(&cv.ty, &ty).is_some() || coerce_holes(&cv.ty, &ty) => defaults.push(Some(cv)),
                        Some(_) => {
                            self.diags.push(err(&f.span, format!("default for field '{}' does not match its type {}", f.name, ty_name(&ty))));
                            defaults.push(None);
                        }
                        None => {
                            self.diags.push(err(&f.span, "field defaults must be constants".into()));
                            defaults.push(None);
                        }
                    }
                } else {
                    defaults.push(None);
                }
                fields.push((f.name.clone(), ty));
            }
            let mut methods = Vec::new();
            let mut statics = Vec::new();
            for m in &td.methods {
                let mkey = format!("{}.{}", td.name, m.name);
                if self.funcs.contains_key(&(module.clone(), mkey.clone())) {
                    self.diags.push(err(&m.span, format!("method '{}' is already defined for type '{}'", m.name, td.name)));
                    continue;
                }
                // Static factory-style method: any method without a leading
                // 'self' parameter is callable on the type itself.
                if m.params.is_empty() || m.params[0].name != "self" {
                    self.funcs.insert((module.clone(), mkey.clone()), Rc::new(m.clone()));
                    self.fn_order.push((module.clone(), mkey.clone()));
                    statics.push(mkey);
                    continue;
                }
                self.funcs.insert((module.clone(), mkey.clone()), Rc::new(m.clone()));
                self.fn_order.push((module.clone(), mkey.clone()));
                methods.push(mkey);
            }
            self.prog.structs.insert(
                qual.clone(),
                StructInfo {
                    module,
                    name: td.name.clone(),
                    fields,
                    defaults,
                    is_soa: td.is_soa(),
                    methods,
                    statics,
                    exported: td.attrs.iter().any(|a| a == "export"),
                    ty_args: Vec::new(),
                },
            );
        }
        for qual in &quals {
            if let Some(bad) = self.struct_cycle(qual, &mut HashSet::new()) {
                let td = &self.types[qual];
                self.diags.push(err(&td.span, format!("type '{}' contains itself by value through field '{}' (use a list or dict to break the cycle)", td.name, bad)));
            }
        }
    }

    fn struct_cycle(&self, qual: &str, seen: &mut HashSet<String>) -> Option<String> {
        if seen.contains(qual) {
            return Some(String::new());
        }
        seen.insert(qual.to_string());
        let si = self.prog.structs.get(qual)?;
        for (fname, fty) in &si.fields {
            if let Ty::Named(inner) = resolve(fty) {
                if let Some(bad) = self.struct_cycle(&inner, seen) {
                    if bad.is_empty() {
                        return Some(fname.clone());
                    }
                    return Some(bad);
                }
            }
        }
        seen.remove(qual);
        None
    }

    /// Register every enum definition: pass 1 inserts the EnumInfo with the
    /// variant NAMES only (payload types resolved in pass 2), so struct
    /// fields may reference enums; pass 2 resolves payload annotations
    /// against the full struct registry, so enum payloads may be structs.
    fn collect_enums(&mut self) {
        let mod_names: Vec<String> = self.mods.keys().cloned().collect();
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            for s in &stmts {
                let ed = match s {
                    Stmt::EnumDef(e) => e,
                    _ => continue,
                };
                let qual = format!("{}::{}", m, ed.name);
                if self.prog.enums.contains_key(&qual) {
                    continue; // pass 1 already registered this enum
                }
                if self.short_types.get(m).map(|t| t.contains_key(&ed.name)).unwrap_or(false) {
                    self.diags.push(err(&ed.span, format!("'{}' is already defined in this module", ed.name)));
                    continue;
                }
                self.prog.enums.insert(
                    qual.clone(),
                    EnumInfo {
                        module: m.clone(),
                        name: ed.name.clone(),
                        variants: Vec::new(),
                        exported: false,
                    },
                );
            }
        }
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            for s in &stmts {
                let ed = match s {
                    Stmt::EnumDef(e) => e,
                    _ => continue,
                };
                let qual = format!("{}::{}", m, ed.name);
                let mut variants: Vec<(String, Vec<Ty>)> = Vec::new();
                for v in &ed.variants {
                    let mut pts = Vec::new();
                    for pa in &v.payloads {
                        match self.annot_ty_in(m, pa, &v.span) {
                            Ok(t) => pts.push(t),
                            Err(msg) => {
                                self.diags.push(err(&v.span, format!("payload of variant '{}' in enum '{}': {}", v.name, ed.name, msg)));
                                pts.push(Ty::Int);
                            }
                        }
                    }
                    variants.push((v.name.clone(), pts));
                }
                if let Some(ei) = self.prog.enums.get_mut(&qual) {
                    ei.variants = variants;
                    ei.exported = false;
                }
            }
        }
    }

    /// Find an enum by short name visible from `module` (own module first,
    /// then `use`d modules). Returns (module, qual).
    fn find_enum(&self, module: &str, name: &str) -> Option<(String, String)> {
        if let Some(q) = self.prog.enums.keys().find(|q| {
            let m = q.split("::").next().unwrap();
            let n = q.split("::").nth(1).unwrap_or("");
            m == module && n == name
        }) {
            return Some((module.to_string(), q.clone()));
        }
        if let Some(als) = self.aliases.get(module) {
            for target in als.values() {
                if let Some(q) = self.prog.enums.keys().find(|q| {
                    let m = q.split("::").next().unwrap();
                    let n = q.split("::").nth(1).unwrap_or("");
                    m == *target && n == name
                }) {
                    return Some((target.clone(), q.clone()));
                }
            }
        }
        None
    }

    /// Register every trait definition with resolved method signatures.
    /// Trait bodies are signature-only; registration happens before struct
    /// collection so impls (which need both) can follow.
    fn collect_traits(&mut self) {
        let mod_names: Vec<String> = self.mods.keys().cloned().collect();
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            for s in &stmts {
                let td = match s {
                    Stmt::TraitDef(t) => t,
                    _ => continue,
                };
                let qual = format!("{}::{}", m, td.name);
                if self.types.contains_key(&qual) {
                    self.diags.push(err(&td.span, format!("'{}' is already defined as a type in this module", td.name)));
                    continue;
                }
                if self.prog.enums.contains_key(&qual) {
                    self.diags.push(err(&td.span, format!("'{}' is already defined as an enum in this module", td.name)));
                    continue;
                }
                let mut methods: Vec<(String, bool, Vec<Ty>, Option<Ty>)> = Vec::new();
                for f in &td.methods {
                    let mut pts: Vec<Ty> = Vec::new();
                    for p in f.params.iter().skip(1) {
                        match self.annot_ty_in(m, p.annot.as_ref().unwrap(), &f.span) {
                            Ok(t) => pts.push(t),
                            Err(msg) => {
                                self.diags.push(err(&f.span, format!("trait '{}': {}", td.name, msg)));
                                pts.push(Ty::Int);
                            }
                        }
                    }
                    let ret = match &f.ret {
                        Some(a) => match self.annot_ty_in(m, a, &f.span) {
                            Ok(t) => Some(t),
                            Err(msg) => {
                                self.diags.push(err(&f.span, format!("trait '{}': {}", td.name, msg)));
                                None
                            }
                        },
                        None => None,
                    };
                    let has_self = !f.params.is_empty() && f.params[0].name == "self";
                    methods.push((f.name.clone(), has_self, pts, ret));
                }
                self.prog.traits.insert(
                    qual.clone(),
                    TraitInfo {
                        module: m.clone(),
                        name: td.name.clone(),
                        methods,
                        span: td.span.clone(),
                    },
                );
            }
        }
    }

    /// Find a trait by short name visible from `module`.
    fn find_trait(&self, module: &str, name: &str) -> Option<String> {
        let own = format!("{}::{}", module, name);
        if self.prog.traits.contains_key(&own) {
            return Some(own);
        }
        if let Some(als) = self.aliases.get(module) {
            for target in als.values() {
                let q = format!("{}::{}", target, name);
                if self.prog.traits.contains_key(&q) {
                    return Some(q);
                }
            }
        }
        None
    }

    /// Validate every `impl Trait for Type` and register its methods as real
    /// struct methods (static dispatch): the same registration a type's own
    /// methods get. Impls must live in the method-lookup module of the type
    /// (its own), and cover every trait method with a signature match.
    fn collect_impls(&mut self) {
        let mod_names: Vec<String> = self.mods.keys().cloned().collect();
        for m in &mod_names {
            let stmts = self.mods[m].clone();
            for s in &stmts {
                let id = match s {
                    Stmt::ImplDef(i) => i,
                    _ => continue,
                };
                let tq = match self.find_trait(m, &id.name) {
                    Some(q) => q,
                    None => {
                        self.diags.push(err(&id.span, format!("unknown trait '{}'", id.name)));
                        continue;
                    }
                };
                // The trait must belong to this module: impls extend the
                // type's own method registry, keyed by its module.
                if !tq.starts_with(&format!("{}::", m)) {
                    self.diags.push(err(&id.span, format!("trait '{}' is defined in another module; impls must be in the trait's module", id.name)));
                    continue;
                }
                let ti = self.prog.traits[&tq].clone();
                // The type must be a non-generic struct of this module.
                let tq_td = match self.find_type(m, &id.for_type) {
                    Some((tm, qual, td)) if tm == *m => {
                        if !td.params.is_empty() {
                            self.diags.push(err(&id.span, format!("impls are not supported for generic type '{}' (instantiate it first)", id.for_type)));
                            continue;
                        }
                        (qual, td)
                    }
                    Some((_, _, _)) => {
                        self.diags.push(err(&id.span, format!("impls must be in the same module as type '{}'", id.for_type)));
                        continue;
                    }
                    None => {
                        self.diags.push(err(&id.span, format!("unknown type '{}'", id.for_type)));
                        continue;
                    }
                };
                let (qual, td) = tq_td;
                if self.prog.enums.contains_key(&qual) {
                    self.diags.push(err(&id.span, format!("impls are not supported for enums ('{}' is an enum)", id.for_type)));
                    continue;
                }
                // Every trait method must be implemented exactly once, with a
                // matching signature; extra methods are rejected.
                let mut covered: HashSet<String> = HashSet::new();
                for im in &id.methods {
                    let sig = ti.methods.iter().find(|(n, _, _, _)| n == &im.name);
                    let sig = match sig {
                        Some(s) => s,
                        None => {
                            let known: Vec<String> = ti.methods.iter().map(|(n, _, _, _)| n.clone()).collect();
                            let hint = did_you_mean(&im.name, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                            self.diags.push(err(&im.span, format!("method '{}' is not declared in trait '{}'{}", im.name, ti.name, hint)));
                            continue;
                        }
                    };
                    if !covered.insert(im.name.clone()) {
                        continue; // duplicate impl method: caught below
                    }
                    // Method-ness and arity must agree with the trait.
                    let im_self = !im.params.is_empty() && im.params[0].name == "self";
                    if im_self != sig.1 {
                        self.diags.push(err(
                            &im.span,
                            format!(
                                "method '{}' does not match trait '{}': the impl {} self but the trait {}",
                                im.name,
                                ti.name,
                                if im_self { "takes" } else { "does not take" },
                                if sig.1 { "does" } else { "does not" }
                            ),
                        ));
                        continue;
                    }
                    let want_len = sig.2.len() + if im_self { 1 } else { 0 };
                    if im.params.len() != want_len {
                        self.diags.push(err(
                            &im.span,
                            format!(
                                "method '{}' does not match trait '{}': takes {} parameter(s) (self included) but the trait declares {}",
                                im.name, ti.name, im.params.len(), want_len
                            ),
                        ));
                        continue;
                    }
                    // Signature types must match (self is skipped on both).
                    let mut sig_ok = true;
                    let off = if im_self { 1 } else { 0 };
                    for (i, p) in im.params.iter().enumerate().skip(off) {
                        let pa = match &p.annot {
                            Some(a) => a,
                            None => {
                                self.diags.push(err(&p.span, format!("impl method '{}' parameter '{}' needs a type annotation to match trait '{}'", im.name, p.name, ti.name)));
                                sig_ok = false;
                                break;
                            }
                        };
                        let pt = match self.annot_ty_in(m, pa, &p.span) {
                            Ok(t) => t,
                            Err(msg) => {
                                self.diags.push(err(&p.span, msg));
                                sig_ok = false;
                                break;
                            }
                        };
                        if !ty_eq(&pt, &sig.2[i - off]) {
                            self.diags.push(err(&p.span, format!("method '{}' does not match trait '{}': parameter '{}' is {} but the trait declares {}", im.name, ti.name, p.name, ty_name(&pt), ty_name(&sig.2[i - off]))));
                            sig_ok = false;
                            break;
                        }
                    }
                    if !sig_ok {
                        continue;
                    }
                    match (&im.ret, &sig.3) {
                        (Some(a), Some(rt)) => match self.annot_ty_in(m, a, &im.span) {
                            Ok(t) => {
                                if !ty_eq(&t, rt) {
                                    self.diags.push(err(&im.span, format!("method '{}' does not match trait '{}': returns {} but the trait declares {}", im.name, ti.name, ty_name(&t), ty_name(rt))));
                                    continue;
                                }
                            }
                            Err(msg) => {
                                self.diags.push(err(&im.span, msg));
                                continue;
                            }
                        },
                        (Some(_), None) => {
                            self.diags.push(err(&im.span, format!("method '{}' does not match trait '{}': has a return type but the trait declares none", im.name, ti.name)));
                            continue;
                        }
                        (None, Some(rt)) => {
                            self.diags.push(err(&im.span, format!("method '{}' does not match trait '{}': must return {}", im.name, ti.name, ty_name(rt))));
                            continue;
                        }
                        (None, None) => {}
                    }
                    // Register like a type method: dotted name under the
                    // module; self-first methods are instance methods.
                    let mkey = format!("{}.{}", td.name, im.name);
                    if self.funcs.contains_key(&(m.clone(), mkey.clone())) {
                        self.diags.push(err(&im.span, format!("method '{}' is already defined for type '{}'", im.name, td.name)));
                        continue;
                    }
                    self.funcs.insert((m.clone(), mkey.clone()), Rc::new(im.clone()));
                    self.fn_order.push((m.clone(), mkey.clone()));
                    if let Some(si) = self.prog.structs.get_mut(&qual) {
                        if im_self {
                            si.methods.push(mkey);
                        } else {
                            si.statics.push(mkey);
                        }
                    }
                }
                // Coverage: every trait method needs an impl definition.
                for (n, _, _, _) in &ti.methods {
                    if !id.methods.iter().any(|im| im.name == *n) {
                        self.diags.push(err(&id.span, format!("impl '{}' for '{}' is missing method '{}'", ti.name, td.name, n)));
                    }
                }
            }
        }
    }

    /// Qual of a type by (module, short name): non-generic types come from
    /// short_types, instantiated generics from inst_by_name.
    fn type_qual_of(&self, module: &str, tname: &str) -> Option<String> {
        if let Some(q) = self.short_types.get(module).and_then(|s| s.get(tname)) {
            return Some(q.clone());
        }
        self.inst_by_name.get(&(module.to_string(), tname.to_string())).cloned()
    }

    /// Find a generic type def by short name visible from `module` (own
    /// module first, then `use`d modules). Returns (module, qual, typedef).
    fn find_type(&self, module: &str, name: &str) -> Option<(String, String, TypeDef)> {
        if let Some(q) = self.short_types.get(module).and_then(|s| s.get(name)) {
            if let Some(td) = self.types.get(q) {
                return Some((module.to_string(), q.clone(), td.clone()));
            }
        }
        if let Some(als) = self.aliases.get(module) {
            for target in als.values() {
                if let Some(q) = self.short_types.get(target).and_then(|s| s.get(name)) {
                    if let Some(td) = self.types.get(q) {
                        return Some((target.clone(), q.clone(), td.clone()));
                    }
                }
            }
        }
        None
    }

    /// Resolve an annotation to a Ty in the context of `module`, with an
    /// optional type-parameter substitution (generic method bodies and field
    /// types of generic instantiations). Instantiates nested generic types
    /// on demand.
    fn annot_ty_subst(&mut self, module: &str, a: &TypeAnnot, params: &[String], args: &[Ty], sp: &Span) -> Result<Ty, String> {
        match a {
            TypeAnnot::Int => Ok(Ty::Int),
            TypeAnnot::Float => Ok(Ty::Float),
            TypeAnnot::Bool => Ok(Ty::Bool),
            TypeAnnot::Str => Ok(Ty::Str),
            TypeAnnot::None => Ok(Ty::NoneTy),
            TypeAnnot::Any => Ok(Ty::Any),
            TypeAnnot::List(e) => Ok(Ty::List(Rc::new(self.annot_ty_subst(module, e, params, args, sp)?))),
            TypeAnnot::Dict(k, v) => Ok(Ty::Dict(
                Rc::new(self.annot_ty_subst(module, k, params, args, sp)?),
                Rc::new(self.annot_ty_subst(module, v, params, args, sp)?),
            )),
            TypeAnnot::Tuple(ts) => Ok(Ty::Tuple(Rc::new(
                ts.iter().map(|t| self.annot_ty_subst(module, t, params, args, sp)).collect::<Result<_, _>>()?,
            ))),
            TypeAnnot::Nullable(b) => {
                let inner = self.annot_ty_subst(module, b, params, args, sp)?;
                if matches!(inner, Ty::Any) {
                    return Err("'any?' is not needed; any can already hold none".into());
                }
                Ok(Ty::Nullable(Box::new(inner)))
            }
            TypeAnnot::Named(n) => {
                if let Some(i) = params.iter().position(|p| p == n) {
                    return Ok(args[i].clone());
                }
                // Enums resolve before structs: both are Ty::Named, keyed by
                // their own registries.
                if let Some((_m, eq)) = self.find_enum(module, n) {
                    return Ok(Ty::Named(eq));
                }
                // A bare generic type name without args is an arity error.
                if let Some((_, q, td)) = self.find_type(module, n) {
                    if !td.params.is_empty() {
                        return Err(format!(
                            "type '{}' takes {} type argument(s) — write {}<{}>",
                            n,
                            td.params.len(),
                            n,
                            td.params.join(", ")
                        ));
                    }
                    let _ = q;
                }
                annot_to_ty(&TypeAnnot::Named(n.clone()), &self.prog.structs)
            }
            TypeAnnot::Generic(name, sub) => {
                let (tmod, _q, td) = self
                    .find_type(module, name)
                    .ok_or_else(|| format!("unknown type '{}'", name))?;
                if td.params.is_empty() {
                    return Err(format!("type '{}' is not generic; write {} instead of {}<...>", name, name, name));
                }
                if sub.len() != td.params.len() {
                    return Err(format!(
                        "type '{}' takes {} type argument(s), got {}",
                        name,
                        td.params.len(),
                        sub.len()
                    ));
                }
                let mut arg_tys = Vec::new();
                for sa in sub {
                    arg_tys.push(self.annot_ty_subst(module, sa, params, args, sp)?);
                }
                match self.instantiate_struct(&tmod, name, &arg_tys, sp) {
                    Some(key) => Ok(Ty::Named(key)),
                    None => Err(format!("cannot instantiate type '{}'", name)),
                }
            }
        }
    }

    /// Module-context annotation resolution with no type parameters in scope.
    fn annot_ty_in(&mut self, module: &str, a: &TypeAnnot, sp: &Span) -> Result<Ty, String> {
        self.annot_ty_subst(module, a, &[], &[], sp)
    }

    /// Convert a resolved Ty back into an annotation (for registering
    /// per-instantiation methods with substituted param/ret types).
    /// None for inference-only types (Var/Never) — the param stays inferred.
    fn annot_of_ty(&self, t: &Ty) -> Option<TypeAnnot> {
        match resolve(t) {
            Ty::Int => Some(TypeAnnot::Int),
            Ty::Float => Some(TypeAnnot::Float),
            Ty::Bool => Some(TypeAnnot::Bool),
            Ty::Str => Some(TypeAnnot::Str),
            Ty::NoneTy => Some(TypeAnnot::None),
            Ty::Any => Some(TypeAnnot::Any),
            Ty::List(e) => Some(TypeAnnot::List(Box::new(self.annot_of_ty(&e)?))),
            Ty::Dict(k, v) => Some(TypeAnnot::Dict(Box::new(self.annot_of_ty(&k)?), Box::new(self.annot_of_ty(&v)?))),
            Ty::Tuple(ts) => Some(TypeAnnot::Tuple(
                ts.iter().map(|t| self.annot_of_ty(t)).collect::<Option<Vec<_>>>()?,
            )),
            Ty::Nullable(b) => Some(TypeAnnot::Nullable(Box::new(self.annot_of_ty(&b)?))),
            Ty::Named(q) => Some(TypeAnnot::Named(q.clone())),
            Ty::Var(_) | Ty::Never | Ty::Func(_, _) => None,
        }
    }

    /// Instantiate a generic type with concrete type args, producing (and
    /// caching) a concrete StructInfo plus per-instantiation methods. Returns
    /// the instantiated qual, e.g. "m::SlotMap<i>".
    fn instantiate_struct(&mut self, tmod: &str, name: &str, arg_tys: &[Ty], sp: &Span) -> Option<String> {
        if self.inst_depth > 32 {
            self.diags.push(err(sp, "generic type instantiation nested too deeply".into()));
            return None;
        }
        let qual = format!("{}::{}", tmod, name);
        let td = match self.types.get(&qual) {
            Some(t) if !t.params.is_empty() => t.clone(),
            _ => {
                self.diags.push(err(sp, format!("unknown generic type '{}'", name)));
                return None;
            }
        };
        if td.is_soa() {
            self.diags.push(err(sp, "@soa types cannot have type parameters".into()));
            return None;
        }
        if arg_tys.len() != td.params.len() {
            self.diags.push(err(sp, format!("type '{}' takes {} type argument(s), got {}", name, td.params.len(), arg_tys.len())));
            return None;
        }
        let tags: Vec<String> = arg_tys.iter().map(ty_tag).collect();
        let key = format!("{}::{}<{}>", tmod, name, tags.join(","));
        if self.prog.structs.contains_key(&key) {
            return Some(key);
        }
        let short_inst = format!("{}<{}>", name, tags.join(","));
        self.inst_depth += 1;
        let mut fields: Vec<(String, Ty)> = Vec::new();
        let mut defaults: Vec<Option<Tx>> = Vec::new();
        let mut methods: Vec<String> = Vec::new();
        let mut statics: Vec<String> = Vec::new();
        let mut ok = true;
        for f in &td.fields {
            let ty = match self.annot_ty_subst(tmod, &f.annot, &td.params, arg_tys, sp) {
                Ok(t) => t,
                Err(m) => {
                    self.diags.push(err(&f.span, m));
                    ok = false;
                    Ty::Int
                }
            };
            if td.is_soa() && !matches!(resolve(&ty), Ty::Int | Ty::Float | Ty::Bool | Ty::Str) {
                self.diags.push(err(&f.span, format!("@soa fields must be int, float, bool or str; field '{}' is {}", f.name, ty_name(&ty))));
            }
            let d = match &f.default {
                Some(dexpr) => match self.const_eval(dexpr) {
                    Some(cv) => {
                        if assign_ok(&cv.ty, &ty).is_some() || coerce_holes(&cv.ty, &ty) {
                            Some(cv)
                        } else if is_empty_literal(dexpr) {
                            // An empty list/dict default unifies with the
                            // field's concrete type.
                            Some(Tx { ty: ty.clone(), span: f.span.clone(), x: empty_literal_x(&ty) })
                        } else {
                            self.diags.push(err(&f.span, format!("default for field '{}' does not match its type {}", f.name, ty_name(&ty))));
                            None
                        }
                    }
                    None => {
                        self.diags.push(err(&f.span, "field defaults must be constants".into()));
                        None
                    }
                },
                None => None,
            };
            fields.push((f.name.clone(), ty));
            defaults.push(d);
        }
        // Insert the (still method-less) instantiation BEFORE registering its
        // methods: a method signature may reference the owner type itself
        // (`func make_ok(v: T) -> Result<T, E>`), whose substitution re-enters
        // instantiate_struct for this same key and must hit the cache. The
        // completed method lists are patched back in below.
        self.prog.structs.insert(
            key.clone(),
            StructInfo {
                module: tmod.to_string(),
                name: short_inst.clone(),
                fields,
                defaults,
                is_soa: false,
                methods: Vec::new(),
                statics: Vec::new(),
                exported: td.attrs.iter().any(|a| a == "export"),
                ty_args: arg_tys.to_vec(),
            },
        );
        self.inst_by_name.insert((tmod.to_string(), short_inst.clone()), key.clone());
        // Per-instantiation methods: params/ret substituited to the concrete
        // types so the ordinary param_info/instantiate machinery handles them.
        for m in &td.methods {
            let mkey = format!("{}.{}", short_inst, m.name);
            if self.funcs.contains_key(&(tmod.to_string(), mkey.clone())) {
                self.diags.push(err(&m.span, format!("method '{}' is already defined for type '{}'", m.name, short_inst)));
                continue;
            }
            let mut sm = m.clone();
            let mut m_ok = true;
            for p in &mut sm.params {
                if let Some(a) = p.annot.clone() {
                    match self.annot_ty_subst(tmod, &a, &td.params, arg_tys, sp) {
                        Ok(t) => {
                            let kept = if p.name == "self" { Some(a) } else { self.annot_of_ty(&t) };
                            p.annot = kept;
                        }
                        Err(ms) => {
                            self.diags.push(err(&p.span, ms));
                            m_ok = false;
                        }
                    }
                }
            }
            if let Some(ra) = m.ret.clone() {
                match self.annot_ty_subst(tmod, &ra, &td.params, arg_tys, sp) {
                    Ok(t) => {
                        let rt = resolve_deep(&t);
                        sm.ret = self.annot_of_ty(&rt);
                        if sm.ret.is_none() {
                            // keep inference (no annotation)
                        }
                    }
                    Err(ms) => {
                        self.diags.push(err(&m.span, ms));
                        m_ok = false;
                    }
                }
            }
            if !m_ok {
                ok = false;
                // Do not register a method whose substituted signature is
                // broken; the primary diagnostic above already fired.
                continue;
            }
            self.funcs.insert((tmod.to_string(), mkey.clone()), Rc::new(sm));
            self.fn_order.push((tmod.to_string(), mkey.clone()));
            if m.params.is_empty() || m.params[0].name != "self" {
                statics.push(mkey);
            } else {
                methods.push(mkey);
            }
        }
        // Patch the instantiation with its completed method lists.
        if let Some(si) = self.prog.structs.get_mut(&key) {
            si.methods = methods;
            si.statics = statics;
        }
        if ok {
            let mut seen = HashSet::new();
            if let Some(bad) = self.struct_cycle(&key, &mut seen) {
                self.diags.push(err(&td.span, format!("type '{}' contains itself by value through field '{}' (use a list or dict to break the cycle)", short_inst, bad)));
            }
        }
        self.inst_depth -= 1;
        if ok {
            Some(key)
        } else {
            None
        }
    }

    fn order_modules(&mut self, main_module: &str) {
        let mut order: Vec<String> = Vec::new();
        let mut state: HashMap<String, u32> = HashMap::new();
        let mods: Vec<String> = self.mods.keys().cloned().collect();
        let deps: HashMap<String, Vec<String>> = mods
            .iter()
            .map(|m| {
                let mut d = Vec::new();
                if let Some(stmts) = self.mods.get(m) {
                    for s in stmts {
                        if let Stmt::Use { target, .. } = s {
                            d.push(target.clone());
                        }
                    }
                }
                (m.clone(), d)
            })
            .collect();
        fn visit(
            m: &str,
            deps: &HashMap<String, Vec<String>>,
            state: &mut HashMap<String, u32>,
            order: &mut Vec<String>,
            diags: &mut Vec<Diag>,
        ) {
            match state.get(m) {
                Some(2) => return,
                Some(1) => {
                    diags.push(err(&Span::new(std::rc::Rc::from("modules"), 1, 1), format!("circular 'use' involving module '{}'", m)));
                    return;
                }
                _ => {}
            }
            state.insert(m.to_string(), 1);
            if let Some(ds) = deps.get(m) {
                for d in ds.clone() {
                    if deps.contains_key(&d) {
                        visit(&d, deps, state, order, diags);
                    }
                }
            }
            state.insert(m.to_string(), 2);
            order.push(m.to_string());
        }
        visit(main_module, &deps, &mut state, &mut order, &mut self.diags);
        for m in mods {
            visit(&m, &deps, &mut state, &mut order, &mut self.diags);
        }
        self.init_order = order;
        self.prog.modules = self.init_order.clone();
    }

    fn check_module_init(&mut self, module: &str) -> Vec<CS> {
        let stmts = self.mods[module].clone();
        let mut ctx = Ctx::new(module.to_string(), false);
        let mut out = Vec::new();
        for s in &stmts {
            self.check_top(&mut ctx, s, &mut out, module);
        }
        out
    }

    fn check_top(&mut self, ctx: &mut Ctx, s: &Stmt, out: &mut Vec<CS>, _module: &str) {
        match s {
            Stmt::FuncDef(_) | Stmt::TypeDef(_) | Stmt::EnumDef(_) | Stmt::TraitDef(_) | Stmt::ImplDef(_) | Stmt::Extern(_) | Stmt::Use { .. } => {}
            _ => {
                let _ = self.check_stmt(ctx, s, out);
            }
        }
    }

    fn param_info(&mut self, module: &str, fname: &str) -> (Vec<Option<Ty>>, Vec<Option<Tx>>) {
        if let Some(v) = self.pcache.get(&(module.to_string(), fname.to_string())) {
            return v.clone();
        }
        let fd = self.funcs[&(module.to_string(), fname.to_string())].clone();
        let mut ptys = Vec::new();
        let mut pdefs = Vec::new();
        for p in &fd.params {
            if p.name == "self" {
                ptys.push(None);
                pdefs.push(None);
                continue;
            }
            let ty = if p.annot.is_none() && p.default.is_some() {
                match self.const_eval(p.default.as_ref().unwrap()) {
                    Some(cv) => Some(cv.ty),
                    None => {
                        self.diags.push(err(&p.span, format!("the default value of '{}' must be a constant", p.name)));
                        None
                    }
                }
            } else {
                p.annot.as_ref().and_then(|a| self.annot_ty_in(module, a, &p.span).ok())
            };
            if ty.is_some() && p.default.is_some() {
                let cv = self.const_eval(p.default.as_ref().unwrap());
                match cv {
                    Some(cv) if assign_ok(&cv.ty, ty.as_ref().unwrap()).is_some() || coerce_holes(&cv.ty, ty.as_ref().unwrap()) => pdefs.push(Some(cv)),
                    _ => {
                        self.diags.push(err(&p.span, "default values must be constants matching the parameter type".into()));
                        pdefs.push(None);
                    }
                }
            } else {
                pdefs.push(None);
            }
            ptys.push(ty);
        }
        self.pcache.insert((module.to_string(), fname.to_string()), (ptys.clone(), pdefs.clone()));
        (ptys, pdefs)
    }

    fn const_eval(&self, e: &Expr) -> Option<Tx> {
        let sp = e.span();
        let x = match e {
            Expr::IntLit(v, _) => X::Int(*v),
            Expr::FloatLit(v, _) => X::Float(*v),
            Expr::BoolLit(v, _) => X::Bool(*v),
            Expr::NoneLit(_) => X::NoneV,
            Expr::StrLit(s, _) => X::Str(s.clone()),
            Expr::Unary(UnOp::Neg, inner, _) => {
                let t = self.const_eval(inner)?;
                match t.x {
                    X::Int(v) => X::Int(-v),
                    X::Float(v) => X::Float(-v),
                    _ => return None,
                }
            }
            Expr::Unary(UnOp::BNot, inner, _) => {
                let t = self.const_eval(inner)?;
                match t.x {
                    X::Int(v) => X::Int(!v),
                    _ => return None,
                }
            }
            Expr::ListLit(items, _) => {
                let mut xs = Vec::new();
                for i in items {
                    xs.push(self.const_eval(i)?);
                }
                let ty = if xs.is_empty() {
                    Ty::List(Rc::new(Ty::Var(Rc::new(std::cell::RefCell::new(None)))))
                } else {
                    let first = resolve(&xs[0].ty);
                    for x in xs[1..].iter() {
                        if !ty_eq(&x.ty, &first) {
                            return None;
                        }
                    }
                    Ty::List(Rc::new(first))
                };
                return Some(Tx { ty, span: sp, x: X::ListLit(xs) });
            }
            Expr::Binary(op, l, r, _) => {
                let lt = self.const_eval(l)?;
                let rt = self.const_eval(r)?;
                match (&lt.x, &rt.x) {
                    (X::Int(a), X::Int(b)) => {
                        let (a, b) = (*a, *b);
                        let v = match op {
                            BinOp::Add => a.checked_add(b)?,
                            BinOp::Sub => a.checked_sub(b)?,
                            BinOp::Mul => a.checked_mul(b)?,
                            BinOp::Div => {
                                if b == 0 {
                                    return None;
                                }
                                return Some(Tx { ty: Ty::Float, span: sp, x: X::Float(a as f64 / b as f64) });
                            }
                            BinOp::FloorDiv => {
                                if b == 0 {
                                    return None;
                                }
                                a.div_euclid(b)
                            }
                            BinOp::Mod => {
                                if b == 0 {
                                    return None;
                                }
                                a.rem_euclid(b)
                            }
                            BinOp::Pow => {
                                if b < 0 || b > 62 {
                                    return None;
                                }
                                a.checked_pow(b as u32)?
                            }
                            BinOp::Shl => {
                                if b < 0 || b > 63 {
                                    return None;
                                }
                                a.checked_shl(b as u32).unwrap_or(i64::MIN)
                            }
                            BinOp::Shr => {
                                if b < 0 || b > 63 {
                                    return None;
                                }
                                a.checked_shr(b as u32).unwrap_or(0)
                            }
                            BinOp::Band => a & b,
                            BinOp::Bor => a | b,
                            BinOp::Bxor => a ^ b,
                        };
                        X::Int(v)
                    }
                    (X::Float(a), X::Float(b)) => {
                        let (a, b) = (*a, *b);
                        let v = match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => a / b,
                            BinOp::FloorDiv => (a / b).floor(),
                            BinOp::Mod => a - b * (a / b).floor(),
                            BinOp::Pow => a.powf(b),
                            _ => return None,
                        };
                        X::Float(v)
                    }
                    (X::Int(_), X::Float(b)) => {
                        let (a, b) = (match lt.x {
                            X::Int(v) => v as f64,
                            _ => 0.0,
                        }, *b);
                        let v = match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => a / b,
                            BinOp::FloorDiv => (a / b).floor(),
                            BinOp::Mod => a - b * (a / b).floor(),
                            BinOp::Pow => a.powf(b),
                            _ => return None,
                        };
                        X::Float(v)
                    }
                    (X::Float(a), X::Int(_)) => {
                        let (a, b) = (*a, match rt.x {
                            X::Int(v) => v as f64,
                            _ => 0.0,
                        });
                        let v = match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            BinOp::Div => a / b,
                            BinOp::FloorDiv => (a / b).floor(),
                            BinOp::Mod => a - b * (a / b).floor(),
                            BinOp::Pow => a.powf(b),
                            _ => return None,
                        };
                        X::Float(v)
                    }
                    (X::Str(a), X::Str(b)) if *op == BinOp::Add => X::Str(format!("{}{}", a, b)),
                    _ => return None,
                }
            }
            _ => return None,
        };
        let ty = match &x {
            X::Int(_) => Ty::Int,
            X::Float(_) => Ty::Float,
            X::Bool(_) => Ty::Bool,
            X::NoneV => Ty::NoneTy,
            X::Str(_) => Ty::Str,
            _ => return None,
        };
        Some(Tx { ty, span: sp, x })
    }

    fn infer(&mut self, ctx: &mut Ctx, e: &Expr) -> Option<Tx> {
        ctx.depth += 1;
        if ctx.depth > 180 {
            ctx.depth -= 1;
            self.diags.push(err(&e.span(), "expression is nested too deeply".into()));
            return None;
        }
        let r = self.infer_inner(ctx, e);
        ctx.depth -= 1;
        r
    }

    fn infer_inner(&mut self, ctx: &mut Ctx, e: &Expr) -> Option<Tx> {
        let sp = e.span();
        match e {
            Expr::IntLit(v, _) => Some(Tx { ty: Ty::Int, span: sp, x: X::Int(*v) }),
            Expr::FloatLit(v, _) => Some(Tx { ty: Ty::Float, span: sp, x: X::Float(*v) }),
            Expr::BoolLit(v, _) => Some(Tx { ty: Ty::Bool, span: sp, x: X::Bool(*v) }),
            Expr::NoneLit(_) => Some(Tx { ty: Ty::NoneTy, span: sp, x: X::NoneV }),
            Expr::StrLit(s, _) => Some(Tx { ty: Ty::Str, span: sp, x: X::Str(s.clone()) }),
            Expr::Interp(parts, _) => {
                let mut acc: Option<Tx> = None;
                for p in parts {
                    let piece = match p {
                        InterpPart::Lit(s) => Tx { ty: Ty::Str, span: sp.clone(), x: X::Str(s.clone()) },
                        InterpPart::Code(ce) => {
                            let v = self.infer(ctx, ce)?;
                            Tx { ty: Ty::Str, span: v.span.clone(), x: X::AsStr(Box::new(v)) }
                        }
                    };
                    acc = Some(match acc {
                        None => piece,
                        Some(a) => Tx { ty: Ty::Str, span: sp.clone(), x: X::Arith(BinOp::Add, Box::new(a), Box::new(piece)) },
                    });
                }
                acc
            }
            Expr::Ident(name, _) => self.infer_ident(ctx, name, &sp),
            Expr::Tuple(items, _) => {
                let mut xs = Vec::new();
                for i in items {
                    xs.push(self.infer(ctx, i)?);
                }
                let ty = Ty::Tuple(Rc::new(xs.iter().map(|t| t.ty.clone()).collect()));
                Some(Tx { ty, span: sp, x: X::TupleLit(xs) })
            }
            Expr::ListLit(items, _) => {
                if items.is_empty() {
                    return Some(Tx {
                        ty: Ty::List(Rc::new(Ty::Var(Rc::new(std::cell::RefCell::new(None))))),
                        span: sp,
                        x: X::ListLit(Vec::new()),
                    });
                }
                let mut xs = Vec::new();
                for i in items {
                    xs.push(self.infer(ctx, i)?);
                }
                let elem = self.widen_elems(&xs, &sp, "list")?;
                Some(Tx { ty: Ty::List(Rc::new(elem)), span: sp, x: X::ListLit(xs) })
            }
            Expr::DictLit(pairs, _) => {
                let mut xs = Vec::new();
                for (k, v) in pairs {
                    let kt = self.infer(ctx, k)?;
                    let vt = self.infer(ctx, v)?;
                    xs.push((kt, vt));
                }
                if xs.is_empty() {
                    return Some(Tx {
                        ty: Ty::Dict(
                            Rc::new(Ty::Var(Rc::new(std::cell::RefCell::new(None)))),
                            Rc::new(Ty::Var(Rc::new(std::cell::RefCell::new(None)))),
                        ),
                        span: sp,
                        x: X::DictLit(Vec::new()),
                    });
                }
                let kt = self.widen_elems(&xs.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(), &sp, "dict key")?;
                let vt = self.widen_elems(&xs.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(), &sp, "dict value")?;
                match resolve(&kt) {
                    Ty::Int | Ty::Str | Ty::Var(_) | Ty::Any => {}
                    other => {
                        self.diags.push(err(&sp, format!("dict keys must be int or str, not {}", ty_name(&other))));
                        return None;
                    }
                }
                Some(Tx { ty: Ty::Dict(Rc::new(kt), Rc::new(vt)), span: sp, x: X::DictLit(xs) })
            }
            Expr::Unary(UnOp::Neg, inner, _) => {
                let v = self.infer(ctx, inner)?;
                match resolve(&v.ty) {
                    Ty::Int => Some(Tx { ty: Ty::Int, span: sp, x: X::Neg(Box::new(v)) }),
                    Ty::Float => Some(Tx { ty: Ty::Float, span: sp, x: X::Neg(Box::new(v)) }),
                    Ty::Var(_) => {
                        self.diags.push(err(&sp, "cannot infer the type of this negation; annotate the value".into()));
                        None
                    }
                    other => {
                        self.diags.push(err(&sp, format!("cannot negate {}", ty_name(&other))));
                        None
                    }
                }
            }
            Expr::Unary(UnOp::BNot, inner, _) => {
                let v = self.infer(ctx, inner)?;
                match resolve(&v.ty) {
                    Ty::Int => Some(Tx { ty: Ty::Int, span: sp, x: X::BNot(Box::new(v)) }),
                    Ty::Var(_) => {
                        self.diags.push(err(&sp, "cannot infer the type of '~'; annotate the value".into()));
                        None
                    }
                    other => {
                        self.diags.push(err(&sp, format!("cannot apply '~' (bitwise not) to {}", ty_name(&other))));
                        None
                    }
                }
            }
            Expr::Not(inner, _) => {
                let v = self.infer(ctx, inner)?;
                Some(Tx { ty: Ty::Bool, span: sp, x: X::Not(Box::new(v)) })
            }
            Expr::Binary(op, l, r, _) => {
                let lt = self.infer(ctx, l)?;
                let rt = self.infer(ctx, r)?;
                self.type_arith(ctx, *op, lt, rt, &sp)
            }
            Expr::Cmp(l, op, r, _) => {
                let lt = self.infer(ctx, l)?;
                let rt = self.infer(ctx, r)?;
                self.type_cmp(*op, lt, rt, &sp)
            }
            Expr::And(l, r, _) => {
                let lt = self.infer(ctx, l)?;
                let rt = self.infer(ctx, r)?;
                let ty = self.unify_pair(&lt.ty, &rt.ty, &sp, "and")?;
                Some(Tx { ty, span: sp, x: X::And(Box::new(lt), Box::new(rt)) })
            }
            Expr::Or(l, r, _) => {
                let lt = self.infer(ctx, l)?;
                let rt = self.infer(ctx, r)?;
                let ty = self.unify_pair(&lt.ty, &rt.ty, &sp, "or")?;
                Some(Tx { ty, span: sp, x: X::Or(Box::new(lt), Box::new(rt)) })
            }
            Expr::In(item, cont, neg, _) => {
                let it = self.infer(ctx, item)?;
                let ct = self.infer(ctx, cont)?;
                let item_any = matches!(resolve(&it.ty), Ty::Any);
                match resolve(&ct.ty) {
                    Ty::List(elem) => {
                        let et = (*elem).clone();
                        let et_any = matches!(resolve(&et), Ty::Any);
                        if !item_any && !et_any && !self.compat(&it.ty, &et) {
                            self.diags.push(err(&sp, format!("'{}' cannot be in a list of {}", ty_name(&it.ty), ty_name(&et))));
                            return None;
                        }
                    }
                    Ty::Dict(k, _) => {
                        let kt = (*k).clone();
                        let kt_any = matches!(resolve(&kt), Ty::Any);
                        if !item_any && !kt_any && !self.compat(&it.ty, &kt) {
                            self.diags.push(err(&sp, format!("dict keys are {}, not {}", ty_name(&kt), ty_name(&it.ty))));
                            return None;
                        }
                    }
                    Ty::Str => {
                        if !item_any && !matches!(resolve(&it.ty), Ty::Str) {
                            self.diags.push(err(&sp, "'in' on strings needs a str on the left".into()));
                            return None;
                        }
                    }
                    Ty::Any => {
                        if !boxable(&it.ty) {
                            self.diags.push(err(&sp, format!("cannot search for {} inside an 'any' value", ty_name(&it.ty))));
                            return None;
                        }
                    }
                    Ty::Var(_) => {
                        self.diags.push(err(&sp, "cannot infer what to search in; annotate the value".into()));
                        return None;
                    }
                    other => {
                        self.diags.push(err(&sp, format!("'in' needs a list, dict or str, not {}", ty_name(&other))));
                        return None;
                    }
                }
                Some(Tx { ty: Ty::Bool, span: sp, x: X::In(Box::new(it), Box::new(ct), *neg) })
            }
            Expr::IsNone(inner, neg, _) => {
                let v = self.infer(ctx, inner)?;
                match resolve(&v.ty) {
                    Ty::Nullable(_) | Ty::NoneTy | Ty::Var(_) | Ty::Any => {}
                    other => {
                        self.diags.push(err(&sp, format!("{} can never be none, so this check is not needed", ty_name(&other))));
                    }
                }
                Some(Tx { ty: Ty::Bool, span: sp, x: X::IsNone(Box::new(v), *neg) })
            }
            Expr::Cond(t, c, els, _) => {
                let ct = self.infer(ctx, c)?;
                let tt = self.infer(ctx, t)?;
                let et = self.infer(ctx, els)?;
                let ty = self.unify_pair(&tt.ty, &et.ty, &sp, "this conditional")?;
                Some(Tx { ty, span: sp, x: X::Cond(Box::new(ct), Box::new(tt), Box::new(et)) })
            }
            Expr::Call(callee, args, _) => self.infer_call(ctx, callee, args, &sp),
            Expr::Index(obj, idx, _) => self.infer_index(ctx, obj, idx, &sp, e),
            Expr::Slice(obj, a, b, c, _) => {
                let ot = self.infer(ctx, obj)?;
                let at = match a {
                    Some(x) => Some(self.want_int(ctx, x, "slice start")?),
                    None => None,
                };
                let bt = match b {
                    Some(x) => Some(self.want_int(ctx, x, "slice end")?),
                    None => None,
                };
                let ct = match c {
                    Some(x) => Some(self.want_int(ctx, x, "slice step")?),
                    None => None,
                };
                match resolve(&ot.ty) {
                    Ty::List(elem) => Some(Tx { ty: Ty::List(elem), span: sp, x: X::Slice(Box::new(ot), at.map(Box::new), bt.map(Box::new), ct.map(Box::new)) }),
                    Ty::Str => Some(Tx { ty: Ty::Str, span: sp, x: X::Slice(Box::new(ot), at.map(Box::new), bt.map(Box::new), ct.map(Box::new)) }),
                    other => {
                        self.diags.push(err(&sp, format!("cannot slice {} (works on str and list)", ty_name(&other))));
                        None
                    }
                }
            }
            Expr::Member(obj, name, _) => self.infer_member(ctx, obj, name, &sp, false),
            Expr::UnwrapQ(inner, _) => {
                let it = self.infer(ctx, inner)?;
                let vt = resolve_deep(&it.ty);
                match vt {
                    Ty::Nullable(b) => {
                        let inner_ty = resolve_deep(&b);
                        if matches!(inner_ty, Ty::NoneTy) {
                            self.diags.push(err(&sp, "'none?' unwraps to none; the value is always none".into()));
                            return None;
                        }
                        Some(Tx { ty: inner_ty, span: sp, x: X::CheckedUnwrap(Box::new(it)) })
                    }
                    other => {
                        self.diags.push(err(&sp, format!("'?' unwraps a nullable value, got {} (which is never none)", ty_name(&other))));
                        None
                    }
                }
            }
        }
    }

    fn want_int(&mut self, ctx: &mut Ctx, e: &Expr, what: &str) -> Option<Tx> {
        let t = self.infer(ctx, e)?;
        match resolve(&t.ty) {
            Ty::Int => Some(t),
            Ty::Var(cell) => {
                *cell.borrow_mut() = Some(Ty::Int);
                Some(t)
            }
            other => {
                self.diags.push(err(&e.span(), format!("{} must be int, not {}", what, ty_name(&other))));
                None
            }
        }
    }

    fn deep_compat(&mut self, a: &Ty, b: &Ty) -> bool {
        let ra = resolve(a);
        let rb = resolve(b);
        if let Ty::Var(c) = &ra {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(rb.clone());
                return true;
            }
        }
        if let Ty::Var(c) = &rb {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(ra.clone());
                return true;
            }
        }
        match (&ra, &rb) {
            (Ty::List(x), Ty::List(y)) => self.deep_compat(x, y),
            (Ty::Dict(xk, xv), Ty::Dict(yk, yv)) => self.deep_compat(xk, yk) && self.deep_compat(xv, yv),
            (Ty::Tuple(x), Ty::Tuple(y)) if x.len() == y.len() => {
                x.iter().zip(y.iter()).all(|(p, q)| self.deep_compat(p, q))
            }
            _ => ty_eq(&ra, &rb) || assign_ok(&ra, &rb).is_some(),
        }
    }

    fn compat(&mut self, a: &Ty, b: &Ty) -> bool {
        let ra = resolve(a);
        let rb = resolve(b);
        if let Ty::Var(cell) = &ra {
            if !matches!(rb, Ty::Var(_)) {
                *cell.borrow_mut() = Some(rb.clone());
                return true;
            }
            return false;
        }
        if let Ty::Var(cell) = &rb {
            *cell.borrow_mut() = Some(ra.clone());
            return true;
        }
        ty_eq(&ra, &rb)
    }

    /// Unify a run of element types for a list/dict literal. When plain
    /// unification fails but every element is a resolved, boxable type,
    /// the literal widens to `any` elements instead (heterogeneous lists
    /// and dicts). Otherwise the original unification error is reported.
    fn widen_elems(&mut self, xs: &[Tx], sp: &Span, what: &str) -> Option<Ty> {
        if xs.is_empty() {
            // `[]` keeps an unbound element hole so `[int] = []` and friends
            // still work; it becomes `any` only if the context demands it.
            return Some(Ty::Var(Rc::new(std::cell::RefCell::new(None))));
        }
        let dmark = self.diags.len();
        let mut elem = xs[0].ty.clone();
        for x in &xs[1..] {
            match self.unify_elems(elem.clone(), x.ty.clone(), sp, what) {
                Some(t) => elem = t,
                None => {
                    self.diags.truncate(dmark);
                    let all_resolved = xs.iter().all(|x| !matches!(resolve(&x.ty), Ty::Var(_)));
                    if all_resolved && xs.iter().all(|x| boxable(&x.ty)) {
                        return Some(Ty::Any);
                    }
                    let mut e2 = xs[0].ty.clone();
                    for x in &xs[1..] {
                        match self.unify_elems(e2.clone(), x.ty.clone(), sp, what) {
                            Some(t) => e2 = t,
                            None => return None,
                        }
                    }
                    return Some(e2);
                }
            }
        }
        Some(elem)
    }

    fn unify_elems(&mut self, a: Ty, b: Ty, sp: &Span, what: &str) -> Option<Ty> {        let ra = resolve(&a);
        let rb = resolve(&b);
        if let Ty::Var(cell) = &ra {
            if matches!(rb, Ty::Var(_)) {
                self.diags.push(err(sp, format!("cannot infer the {} type; add an annotation", what)));
                return None;
            }
            *cell.borrow_mut() = Some(rb.clone());
            return Some(rb);
        }
        if let Ty::Var(cell) = &rb {
            *cell.borrow_mut() = Some(ra.clone());
            return Some(ra);
        }
        match (&ra, &rb) {
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Some(Ty::Float),
            (Ty::NoneTy, other) => Some(Ty::Nullable(Box::new(other.clone()))),
            (other, Ty::NoneTy) => Some(Ty::Nullable(Box::new(other.clone()))),
            _ if ty_eq(&ra, &rb) => Some(ra),
            _ => {
                self.diags.push(err(sp, format!("mixed {} types: {} and {}", what, ty_name(&ra), ty_name(&rb))));
                None
            }
        }
    }

    fn unify_pair(&mut self, a: &Ty, b: &Ty, sp: &Span, what: &str) -> Option<Ty> {
        let ra = resolve(a);
        let rb = resolve(b);
        if ty_eq(&ra, &rb) {
            return Some(ra);
        }
        match (&ra, &rb) {
            (Ty::NoneTy, other) | (other, Ty::NoneTy) => Some(Ty::Nullable(Box::new(other.clone()))),
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Some(Ty::Float),
            (Ty::Nullable(x), other) | (other, Ty::Nullable(x)) if ty_eq(x, other) => Some(Ty::Nullable(Box::new(other.clone()))),
            _ => {
                self.diags.push(err(sp, format!("{} needs matching types on both sides, got {} and {}", what, ty_name(&ra), ty_name(&rb))));
                None
            }
        }
    }

    fn type_arith(&mut self, _ctx: &mut Ctx, op: BinOp, lt: Tx, rt: Tx, sp: &Span) -> Option<Tx> {
        let mut lt = lt;
        let mut rt = rt;
        if let (Ty::Var(a), Ty::Var(b)) = (resolve(&lt.ty), resolve(&rt.ty)) {
            if std::rc::Rc::ptr_eq(&a, &b) {
                *a.borrow_mut() = Some(Ty::Int);
            }
        }
        if let Ty::Var(cell) = resolve(&lt.ty) {
            match resolve(&rt.ty) {
                t @ (Ty::Int | Ty::Float | Ty::Str) => {
                    *cell.borrow_mut() = Some(t.clone());
                    lt.ty = t;
                }
                _ => {}
            }
        }
        if let Ty::Var(cell) = resolve(&rt.ty) {
            match resolve(&lt.ty) {
                t @ (Ty::Int | Ty::Float | Ty::Str) => {
                    *cell.borrow_mut() = Some(t.clone());
                    rt.ty = t;
                }
                _ => {}
            }
        }
        let l = resolve(&lt.ty);
        let r = resolve(&rt.ty);
        if matches!(l, Ty::Any) || matches!(r, Ty::Any) {
            if matches!(l, Ty::NoneTy) || matches!(r, Ty::NoneTy) {
                self.diags.push(err(sp, "cannot use none in arithmetic; check for none first".into()));
                return None;
            }
            return Some(Tx { ty: Ty::Any, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) });
        }
        let num2 = |lc: bool, rc: bool, out: &Ty| -> Option<Tx> {
            let l2 = if lc {
                Tx { ty: Ty::Float, span: lt.span.clone(), x: X::AsFloat(Box::new(lt.clone())) }
            } else {
                lt.clone()
            };
            let r2 = if rc {
                Tx { ty: Ty::Float, span: rt.span.clone(), x: X::AsFloat(Box::new(rt.clone())) }
            } else {
                rt.clone()
            };
            Some(Tx { ty: out.clone(), span: sp.clone(), x: X::Arith(op, Box::new(l2), Box::new(r2)) })
        };
        match op {
            BinOp::Add => match (&l, &r) {
                (Ty::Int, Ty::Int) => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                (Ty::Int, Ty::Float) => num2(true, false, &Ty::Float),
                (Ty::Float, Ty::Int) => num2(false, true, &Ty::Float),
                (Ty::Float, Ty::Float) => num2(false, false, &Ty::Float),
                (Ty::Str, Ty::Str) => Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                (Ty::List(a), Ty::List(b)) => {
                    if !ty_eq(a, b) {
                        self.diags.push(err(sp, format!("cannot concat [{}] with [{}]", ty_name(a), ty_name(b))));
                        return None;
                    }
                    Some(Tx { ty: Ty::List(a.clone()), span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) })
                }
                (Ty::NoneTy, _) | (_, Ty::NoneTy) => {
                    self.diags.push(err(sp, "cannot add none; check for none first".into()));
                    None
                }
                (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                    self.diags.push(err(sp, "cannot infer types for '+' here; annotate the function parameters".into()));
                    None
                }
                _ => {
                    self.diags.push(err(sp, format!("cannot add {} and {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::Sub | BinOp::Mul => match (&l, &r) {
                (Ty::Int, Ty::Int) => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                (Ty::Int, Ty::Float) => num2(true, false, &Ty::Float),
                (Ty::Float, Ty::Int) => num2(false, true, &Ty::Float),
                (Ty::Float, Ty::Float) => num2(false, false, &Ty::Float),
                (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                    self.diags.push(err(sp, format!("cannot infer types for this operation; annotate the values")));
                    None
                }
                _ => {
                    self.diags.push(err(sp, format!("cannot use this operator on {} and {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::Div => match (&l, &r) {
                (Ty::Int, Ty::Int) => num2(true, true, &Ty::Float),
                (Ty::Int, Ty::Float) => num2(true, false, &Ty::Float),
                (Ty::Float, Ty::Int) => num2(false, true, &Ty::Float),
                (Ty::Float, Ty::Float) => num2(false, false, &Ty::Float),
                (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                    self.diags.push(err(sp, "cannot infer types for '/'; annotate the values".into()));
                    None
                }
                _ => {
                    self.diags.push(err(sp, format!("cannot divide {} by {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::FloorDiv | BinOp::Mod => match (&l, &r) {
                (Ty::Int, Ty::Int) => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                (Ty::Int, Ty::Float) => num2(true, false, &Ty::Float),
                (Ty::Float, Ty::Int) => num2(false, true, &Ty::Float),
                (Ty::Float, Ty::Float) => num2(false, false, &Ty::Float),
                (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                    self.diags.push(err(sp, "cannot infer types here; annotate the values".into()));
                    None
                }
                _ => {
                    self.diags.push(err(sp, format!("cannot use this operator on {} and {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::Shl | BinOp::Shr => match (&l, &r) {
                (Ty::Int, Ty::Int) => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                _ => {
                    self.diags.push(err(sp, format!("shift operators need two ints; got {} and {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::Band | BinOp::Bor | BinOp::Bxor => match (&l, &r) {
                (Ty::Int, Ty::Int) => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                (Ty::Bool, Ty::Bool) => Some(Tx { ty: Ty::Bool, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) }),
                _ => {
                    self.diags.push(err(sp, format!("bitwise operators need int (or bool) values; got {} and {}", ty_name(&l), ty_name(&r))));
                    None
                }
            },
            BinOp::Pow => {
                let int_pow = matches!((&l, &r), (Ty::Int, Ty::Int)) && matches!(&rt.x, X::Int(k) if *k >= 0);
                if int_pow {
                    Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Arith(op, Box::new(lt), Box::new(rt)) })
                } else {
                    match (&l, &r) {
                        (Ty::Int, Ty::Int) | (Ty::Int, Ty::Float) => num2(true, false, &Ty::Float),
                        (Ty::Float, Ty::Int) | (Ty::Float, Ty::Float) => num2(false, true, &Ty::Float),
                        (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                            self.diags.push(err(sp, "cannot infer types for '**'; annotate the values".into()));
                            None
                        }
                        _ => {
                            self.diags.push(err(sp, format!("cannot raise {} to {}", ty_name(&l), ty_name(&r))));
                            None
                        }
                    }
                }
            }
        }
    }

    fn type_cmp(&mut self, op: CmpOp, lt: Tx, rt: Tx, sp: &Span) -> Option<Tx> {
        let mut lt = lt;
        let mut rt = rt;
        if let Ty::Var(cell) = resolve(&lt.ty) {
            match resolve(&rt.ty) {
                t @ (Ty::Int | Ty::Float | Ty::Str) => {
                    *cell.borrow_mut() = Some(t.clone());
                    lt.ty = t;
                }
                _ => {}
            }
        }
        if let Ty::Var(cell) = resolve(&rt.ty) {
            match resolve(&lt.ty) {
                t @ (Ty::Int | Ty::Float | Ty::Str) => {
                    *cell.borrow_mut() = Some(t.clone());
                    rt.ty = t;
                }
                _ => {}
            }
        }
        let l = resolve(&lt.ty);
        let r = resolve(&rt.ty);
        let ordered = matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge);
        let ok = match (&l, &r) {
            (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float) | (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => true,
            (Ty::Str, Ty::Str) => true,
            (Ty::Bool, Ty::Bool) => !ordered,
            (Ty::NoneTy, Ty::Nullable(_)) | (Ty::Nullable(_), Ty::NoneTy) => !ordered,
            (Ty::Nullable(a), Ty::Nullable(b)) => ty_eq(a, b) && !ordered,
            (Ty::List(a), Ty::List(b)) => ty_eq(a, b) && !ordered,
            (Ty::Dict(ak, av), Ty::Dict(bk, bv)) => ty_eq(ak, bk) && ty_eq(av, bv) && !ordered,
            (Ty::Tuple(a), Ty::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| ty_eq(x, y)) && !ordered,
            (Ty::Named(a), Ty::Named(b)) => a == b,
            (Ty::Never, _) | (_, Ty::Never) => true,
            (Ty::Any, _) | (_, Ty::Any) => true,
            _ => false,
        };
        if !ok {
            self.diags.push(err(sp, format!("cannot compare {} with {}", ty_name(&l), ty_name(&r))));
            return None;
        }
        if matches!((&l, &r), (Ty::Int, Ty::Float)) {
            lt = Tx { ty: Ty::Float, span: lt.span.clone(), x: X::AsFloat(Box::new(lt)) };
        }
        if matches!((&l, &r), (Ty::Float, Ty::Int)) {
            rt = Tx { ty: Ty::Float, span: rt.span.clone(), x: X::AsFloat(Box::new(rt)) };
        }
        Some(Tx { ty: Ty::Bool, span: sp.clone(), x: X::Cmp(op, Box::new(lt), Box::new(rt)) })
    }

    fn infer_ident(&mut self, ctx: &mut Ctx, name: &str, sp: &Span) -> Option<Tx> {
        if let Some(id) = ctx.lookup(name) {
            let ty = ctx.locals[id as usize].1.clone();
            if ctx.refined.contains(name) {
                if let Ty::Nullable(inner) = resolve(&ty) {
                    return Some(Tx { ty: (*inner).clone(), span: sp.clone(), x: X::Unwrap(Box::new(Tx { ty, span: sp.clone(), x: X::Local(id) })) });
                }
            }
            return Some(Tx { ty, span: sp.clone(), x: X::Local(id) });
        }
        let module = ctx.module.clone();
        if let Some(cname) = self.gmap.get(&(module.clone(), name.to_string())) {
            let ty = self.prog.globals[cname].ty.clone();
            return Some(Tx { ty, span: sp.clone(), x: X::Global(cname.clone()) });
        }
        if let Some((tmod, tname)) = self.imports.get(&module).and_then(|m| m.get(name)).cloned() {
            if let Some(cname) = self.gmap.get(&(tmod.clone(), tname.clone())) {
                let ty = self.prog.globals[cname].ty.clone();
                return Some(Tx { ty, span: sp.clone(), x: X::Global(cname.clone()) });
            }
            if self.funcs.contains_key(&(tmod.clone(), tname.clone())) {
                // Function used as a value: return a function pointer type.
                let tmod_c = tmod.clone();
                let tname_c = tname.clone();
                let fd = self.funcs[&(tmod.clone(), tname.clone())].clone();
                let param_tys: Vec<Ty> = fd.params.iter().map(|p| {
                    if p.name == "self" {
                        // self type comes from the function's self_ty
                        Ty::Named(tname_c.split('.').next().unwrap_or(&tname_c).to_string())
                    } else {
                        p.annot.as_ref().and_then(|a| self.annot_ty_in(&tmod_c, a, &p.span).ok()).unwrap_or(Ty::Int)
                    }
                }).collect();
                let ret_ty = fd.ret.as_ref().and_then(|a| self.annot_ty_in(&tmod_c, a, &fd.span).ok()).unwrap_or(Ty::NoneTy);
                return Some(Tx { ty: Ty::Func(param_tys, Box::new(ret_ty)), span: sp.clone(), x: X::FuncValue(format!("still_{}__{}", sanitize(&tmod_c), sanitize(&tname_c.replace('.', "_")))) });
            }
            self.diags.push(err(sp, format!("'{}' is a type, not a value", name)));
            return None;
        }
        if self.funcs.contains_key(&(module.clone(), name.to_string())) {
            let fd = self.funcs[&(module.clone(), name.to_string())].clone();
            let param_tys: Vec<Ty> = fd.params.iter().map(|p| {
                if p.name == "self" {
                    Ty::Named(name.split('.').next().unwrap_or(name).to_string())
                } else {
                    p.annot.as_ref().and_then(|a| self.annot_ty_in(&module, a, &p.span).ok()).unwrap_or(Ty::Int)
                }
            }).collect();
            let ret_ty = fd.ret.as_ref().and_then(|a| self.annot_ty_in(&module, a, &fd.span).ok()).unwrap_or(Ty::NoneTy);
            return Some(Tx { ty: Ty::Func(param_tys, Box::new(ret_ty)), span: sp.clone(), x: X::FuncValue(format!("still_{}__{}", sanitize(&module), sanitize(&name.replace('.', "_")))) });
        }
        let mut candidates: Vec<String> = ctx.locals.iter().map(|(n, _)| n.clone()).collect();
        for (k, v) in &self.gmap {
            if k.0 == module {
                candidates.push(v.clone());
            }
        }
        for k in self.funcs.keys() {
            if k.0 == module {
                candidates.push(k.1.clone());
            }
        }
        candidates.extend(BUILTINS.iter().map(|s| s.to_string()));
        let hint = did_you_mean(name, &candidates).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
        self.diags.push(err(sp, format!("unknown name '{}'{}", name, hint)));
        None
    }

    fn infer_index(&mut self, ctx: &mut Ctx, obj: &Expr, idx: &Expr, sp: &Span, _whole: &Expr) -> Option<Tx> {
        if let Expr::Member(o, fname, _) = obj {
            let ot = self.infer(ctx, o)?;
            if let Ty::Named(qual) = resolve(&ot.ty) {
                let si = self.prog.structs.get(&qual).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return self.soa_elem_read(ctx, o, &fname, idx, sp, &si);
                    }
                }
            }
        }
        let ot = self.infer(ctx, obj)?;
        let it = self.infer(ctx, idx)?;
        match resolve(&ot.ty) {
            Ty::Dict(k, v) => {
                if !self.compat(&it.ty, &k) {
                    self.diags.push(err(sp, format!("this dict is keyed by {}, not {}", ty_name(&k), ty_name(&it.ty))));
                    return None;
                }
                Some(Tx { ty: (*v).clone(), span: sp.clone(), x: X::Index(Box::new(ot), Box::new(it)) })
            }
            Ty::Any => {
                let it = match resolve(&it.ty) {
                    Ty::Int | Ty::Str | Ty::Never => it,
                    Ty::Var(cell) => {
                        *cell.borrow_mut() = Some(Ty::Int);
                        it
                    }
                    other => {
                        self.diags.push(err(sp, format!("'any' values are indexed by int or str, not {}", ty_name(&other))));
                        return None;
                    }
                };
                Some(Tx { ty: Ty::Any, span: sp.clone(), x: X::Index(Box::new(ot), Box::new(it)) })
            }
            _ => {
                let it = match resolve(&it.ty) {
                    Ty::Int => it,
                    Ty::Var(cell) => {
                        *cell.borrow_mut() = Some(Ty::Int);
                        it
                    }
                    other => {
                        self.diags.push(err(sp, format!("indexes must be int, not {}", ty_name(&other))));
                        return None;
                    }
                };
                match resolve(&ot.ty) {
                    Ty::List(elem) => Some(Tx { ty: (*elem).clone(), span: sp.clone(), x: X::Index(Box::new(ot), Box::new(it)) }),
                    Ty::Str => Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Index(Box::new(ot), Box::new(it)) }),
                    Ty::Tuple(ts) => match &idx {
                        Expr::IntLit(k, _) => {
                            let n = *k as usize;
                            if n >= ts.len() {
                                self.diags.push(err(sp, format!("this tuple has {} values, index {} is out of range", ts.len(), k)));
                                return None;
                            }
                            Some(Tx { ty: ts[n].clone(), span: sp.clone(), x: X::Index(Box::new(ot), Box::new(it)) })
                        }
                        _ => {
                            self.diags.push(err(sp, "tuple indexes must be number literals, like t.0".into()));
                            None
                        }
                    },
                    other => {
                        self.diags.push(err(sp, format!("cannot index {}", ty_name(&other))));
                        None
                    }
                }
            }
        }
    }

    fn soa_elem_read(&mut self, ctx: &mut Ctx, soa_expr: &Expr, fname: &str, idx: &Expr, sp: &Span, si: &StructInfo) -> Option<Tx> {
        let ot = self.infer(ctx, soa_expr)?;
        let it = self.want_int(ctx, idx, "soa index")?;
        let fty = si.fields.iter().find(|(n, _)| n == fname).map(|(_, t)| t.clone());
        match fty {
            Some(t) => Some(Tx { ty: t, span: sp.clone(), x: X::Index(Box::new(Tx { ty: Ty::Named(format!("{}::{}", si.module, si.name)), span: ot.span.clone(), x: X::Field(Box::new(ot), fname.to_string()) }), Box::new(it)) }),
            None => {
                let fields: Vec<String> = si.fields.iter().map(|(n, _)| n.clone()).collect();
                let hint = did_you_mean(fname, &fields).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                self.diags.push(err(sp, format!("type '{}' has no field '{}'{}", si.name, fname, hint)));
                None
            }
        }
    }

    fn infer_member(&mut self, ctx: &mut Ctx, obj: &Expr, name: &str, sp: &Span, _as_value: bool) -> Option<Tx> {
        if let Expr::Ident(base, bsp) = obj {
            let module = ctx.module.clone();
            if ctx.lookup(base).is_none() {
                // Zero-payload variant as a plain value: `Shape.Point`.
                if let Some((_emod, eq)) = self.find_enum(&module, base) {
                    let ei = self.prog.enums.get(&eq).cloned().unwrap();
                    match ei.variant_index(name) {
                        Some(idx) => {
                            if ei.variants[idx].1.is_empty() {
                                return Some(Tx { ty: Ty::Named(eq.clone()), span: sp.clone(), x: X::EnumLit(eq, idx, vec![]) });
                            }
                            self.diags.push(err(bsp, format!("variant '{}' has payloads; construct it with {}.{}(...)", name, ei.name, name)));
                            return None;
                        }
                        None => {
                            self.diags.push(err(bsp, format!("enum '{}' has no variant '{}'", ei.name, name)));
                            return None;
                        }
                    }
                }
                if let Some(target) = self.aliases.get(&module).and_then(|m| m.get(base)).cloned() {
                    if let Some(cname) = self.gmap.get(&(target.clone(), name.to_string())) {
                        let ty = self.prog.globals[cname].ty.clone();
                        return Some(Tx { ty, span: sp.clone(), x: X::Global(cname.clone()) });
                    }
                    self.diags.push(err(bsp, format!("module '{}' has no value named '{}'", target, name)));
                    return None;
                }
            }
        }
        let ot = self.infer(ctx, obj)?;
        match resolve(&ot.ty) {
            Ty::Named(qual) => {
                if let Some(ei) = self.prog.enums.get(&qual).cloned() {
                    if name == "tag" {
                        return Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Field(Box::new(ot), "tag".to_string()) });
                    }
                    for (_vi, (vn, pts)) in ei.variants.iter().enumerate() {
                        for pi in 0..pts.len() {
                            let fname = enum_payload_field(vn, pi);
                            if name == fname {
                                return Some(Tx { ty: pts[pi].clone(), span: sp.clone(), x: X::Field(Box::new(ot), fname) });
                            }
                        }
                    }
                    let mut known = vec!["tag".to_string()];
                    for (vn, pts) in &ei.variants {
                        for pi in 0..pts.len() {
                            known.push(enum_payload_field(vn, pi));
                        }
                    }
                    let hint = did_you_mean(name, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                    self.diags.push(err(sp, format!("enum '{}' has no field '{}'{} (payloads are reached with match patterns)", ei.name, name, hint)));
                    return None;
                }
                let si = self.prog.structs.get(&qual).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        self.diags.push(err(sp, format!("soa field '{}' needs an index: parts.{}[i]", name, name)));
                        return None;
                    }
                    for (n, t) in &si.fields {
                        if n == name {
                            return Some(Tx { ty: t.clone(), span: sp.clone(), x: X::Field(Box::new(ot), name.to_string()) });
                        }
                    }
                    let fields: Vec<String> = si.fields.iter().map(|(n, _)| n.clone()).collect();
                    let hint = did_you_mean(name, &fields).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                    self.diags.push(err(sp, format!("type '{}' has no field '{}'{}", si.name, name, hint)));
                    None
                } else {
                    self.diags.push(err(sp, format!("unknown type '{}'", qual)));
                    None
                }
            }
            other => {
                self.diags.push(err(sp, format!("{} has no field '{}' (fields belong to types; methods are called with .{})", ty_name(&other), name, name)));
                None
            }
        }
    }
}

enum BR {
    No,
    Yes(Option<Tx>),
}

impl Checker {
    fn infer_call(&mut self, ctx: &mut Ctx, callee: &Expr, args: &[Expr], sp: &Span) -> Option<Tx> {
        match callee {
            Expr::Ident(name, nsp) => {
                if let Some(id) = ctx.lookup(name) {
                    let ty = ctx.locals[id as usize].1.clone();
                    if let Ty::Func(param_tys, ret_ty) = resolve(&ty) {
                        // Function value call: f(x, y)
                        let mut ax = Vec::new();
                        for a in args {
                            ax.push(self.infer(ctx, a)?);
                        }
                        // Check argument count
                        if ax.len() != param_tys.len() {
                            self.diags.push(err(sp, format!("function value takes {} argument(s), got {}", param_tys.len(), ax.len())));
                            return None;
                        }
                        let fv = Tx { ty: ty.clone(), span: nsp.clone(), x: X::Local(id) };
                        return Some(Tx { ty: (*ret_ty).clone(), span: sp.clone(), x: X::CallFuncValue(Box::new(fv), ax) });
                    }
                    self.diags.push(err(nsp, format!("cannot call the variable '{}'", name)));
                    return None;
                }
                // Check if it's a global variable of function type
                let module = ctx.module.clone();
                if let Some(cname) = self.gmap.get(&(module.clone(), name.to_string())).cloned() {
                    let ty = self.prog.globals[&cname].ty.clone();
                    if let Ty::Func(param_tys, ret_ty) = resolve(&ty) {
                        let mut ax = Vec::new();
                        for a in args {
                            ax.push(self.infer(ctx, a)?);
                        }
                        if ax.len() != param_tys.len() {
                            self.diags.push(err(sp, format!("function value takes {} argument(s), got {}", param_tys.len(), ax.len())));
                            return None;
                        }
                        let fv = Tx { ty: ty.clone(), span: nsp.clone(), x: X::Global(cname.clone()) };
                        return Some(Tx { ty: (*ret_ty).clone(), span: sp.clone(), x: X::CallFuncValue(Box::new(fv), ax) });
                    }
                }
                let mut ax = Vec::new();
                for a in args {
                    ax.push(self.infer(ctx, a)?);
                }
                if let Some((tmod, tname)) = self.imports.get(&module).and_then(|m| m.get(name)).cloned() {
                    if self.funcs.contains_key(&(tmod.clone(), tname.clone())) {
                        return self.call_fn(ctx, &tmod, &tname, ax, sp);
                    }
                    if let Some(qual) = self.short_types.get(&tmod).and_then(|s| s.get(&tname)).cloned() {
                        return self.ctor(&qual, ax, sp);
                    }
                    if self.externs.contains_key(&tname) {
                        return self.extern_call(&tname, ax, sp);
                    }
                    self.diags.push(err(sp, format!("'{}' cannot be called", name)));
                    return None;
                }
                if self.aliases.get(&module).map(|a| a.contains_key(name)).unwrap_or(false) {
                    self.diags.push(err(nsp, format!("'{}' is a module; call things inside it like {}.func(...)", name, name)));
                    return None;
                }
                if self.funcs.contains_key(&(module.clone(), name.clone())) {
                    return self.call_fn(ctx, &module, name, ax, sp);
                }
                if name.contains('<') {
                    return self.generic_ctor_call(ctx, name, ax, sp);
                }
                if let Some(qual) = self.short_types.get(&module).and_then(|s| s.get(name)).cloned() {
                    // A generic type reached without type args: inside one of
                    // its own method bodies this means the owner instantiation.
                    if let Some(key) = self.owner_ctor(ctx, &module, name, sp) {
                        return self.ctor(&key, ax, sp);
                    }
                    return self.ctor(&qual, ax, sp);
                }
                if self.externs.contains_key(name) {
                    return self.extern_call(name, ax, sp);
                }
                // `use other` makes that module's public types/functions reachable without qualification
                if let Some(als) = self.aliases.get(&module).cloned() {
                    for target in als.values() {
                        if let Some(qual) = self.short_types.get(target).and_then(|s| s.get(name)).cloned() {
                            return self.ctor(&qual, ax, sp);
                        }
                        if self.funcs.contains_key(&(target.clone(), name.clone())) {
                            return self.call_fn(ctx, target, name, ax, sp);
                        }
                        // static methods registered as "Type.method" — e.g. from_km after `use units`
                        let suffix = format!(".{}", name);
                        if let Some(mkey) = self.funcs.keys().find(|(m, n)| *m == *target && n.ends_with(&suffix)).map(|(_, n)| n.clone()) {
                            return self.call_fn(ctx, target, &mkey, ax, sp);
                        }
                    }
                }
                match self.builtin_call(ctx, name, ax, sp) {
                    BR::Yes(Some(t)) => return Some(t),
                    BR::Yes(None) => return None,
                    BR::No => {}
                }
                let mut candidates: Vec<String> = self
                    .funcs
                    .keys()
                    .filter(|(m, _)| *m == module)
                    .map(|(_, n)| n.clone())
                    .collect();
                candidates.extend(self.externs.keys().cloned());
                candidates.extend(BUILTINS.iter().map(|s| s.to_string()));
                for s in self.short_types.get(&module).map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
                    candidates.push(s);
                }
                let hint = did_you_mean(name, &candidates).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                self.diags.push(err(nsp, format!("unknown function '{}'{}", name, hint)));
                None
            }
            Expr::Member(obj, mname, msp) => {
                if let Expr::Ident(base, _) = obj.as_ref() {
                    let module = ctx.module.clone();
                    if ctx.lookup(base).is_none() {
                        // Enum constructor: `Shape.Circle(1.0, 2.0)` — the
                        // base names an enum, the member a variant.
                        if let Some((_emod, eq)) = self.find_enum(&module, base) {
                            return self.enum_ctor(&eq, mname, ctx, args, sp, msp);
                        }
                        // Static method on an instantiated generic type:
                        // SlotMap<int>.new() — the parser encodes the type
                        // args into the base ident string.
                        if base.contains('<') {
                            let ga = match parse_annot_str(base) {
                                Some(TypeAnnot::Generic(b, args)) => Some((b, args)),
                                Some(other) => {
                                    self.diags.push(err(msp, format!("'{}' is not a generic type", annot_str(&other))));
                                    return None;
                                }
                                None => {
                                    self.diags.push(err(msp, format!("malformed type arguments in '{}'", base)));
                                    return None;
                                }
                            };
                            if let Some((gbase, argannots)) = ga {
                                let (tmod, _q, td) = match self.find_type(&module, &gbase) {
                                    Some(t) => t,
                                    None => {
                                        self.diags.push(err(msp, format!("unknown type '{}'", gbase)));
                                        return None;
                                    }
                                };
                                if td.params.is_empty() {
                                    self.diags.push(err(msp, format!("type '{}' is not generic; write {} instead of {}<...>", gbase, gbase, gbase)));
                                    return None;
                                }
                                let mut arg_tys = Vec::new();
                                for sa in &argannots {
                                    match self.annot_ty_subst(&module, sa, &ctx.ty_params, &ctx.ty_args, msp) {
                                        Ok(t) => arg_tys.push(t),
                                        Err(m) => {
                                            self.diags.push(err(msp, m));
                                            return None;
                                        }
                                    }
                                }
                                let key = match self.instantiate_struct(&tmod, &gbase, &arg_tys, msp) {
                                    Some(k) => k,
                                    None => return None,
                                };
                                let si = self.prog.structs.get(&key).cloned();
                                if let Some(si) = si {
                                    let mkey = format!("{}.{}", si.name, mname);
                                    if self.funcs.contains_key(&(si.module.clone(), mkey.clone())) {
                                        let mut ax = Vec::new();
                                        for a in args {
                                            ax.push(self.infer(ctx, a)?);
                                        }
                                        return self.call_fn(ctx, &si.module, &mkey, ax, sp);
                                    }
                                    self.diags.push(err(msp, format!("type '{}' has no static method '{}' (static methods are callable on the type name directly)", si.name, mname)));
                                    return None;
                                }
                                return None;
                            }
                        }
                        // Static method on a type itself: Length.from_km(5.0).
                        // The type may be local or come from a `use`d module.
                        let mut qual: Option<(String, String)> = None;
                        if let Some(q) = self.short_types.get(&module).and_then(|s| s.get(base)).cloned() {
                            qual = Some((module.clone(), q));
                        } else if let Some(als) = self.aliases.get(&module).cloned() {
                            for tgt in als.values() {
                                if let Some(q) = self.short_types.get(tgt).and_then(|s| s.get(base)).cloned() {
                                    qual = Some((tgt.clone(), q));
                                    break;
                                }
                            }
                        }
                        if let Some((tmod, q)) = qual {
                            let tname = q.split("::").nth(1).unwrap_or(base).to_string();
                            let mkey = format!("{}.{}", tname, mname);
                            if self.funcs.contains_key(&(tmod.clone(), mkey.clone())) {
                                let mut ax = Vec::new();
                                for a in args {
                                    ax.push(self.infer(ctx, a)?);
                                }
                                return self.call_fn(ctx, &tmod, &mkey, ax, sp);
                            }
                            self.diags.push(err(msp, format!("type '{}' has no static method '{}' (static methods are callable on the type name directly)", base, mname)));
                            return None;
                        }
                        if let Some(target) = self.aliases.get(&module).and_then(|m| m.get(base)).cloned() {
                            let mut ax = Vec::new();
                            for a in args {
                                ax.push(self.infer(ctx, a)?);
                            }
                            if self.funcs.contains_key(&(target.clone(), mname.clone())) {
                                return self.call_fn(ctx, &target, mname, ax, sp);
                            }
                            if let Some(qual) = self.short_types.get(&target).and_then(|s| s.get(mname)).cloned() {
                                return self.ctor(&qual, ax, sp);
                            }
                            if self.externs.contains_key(mname) {
                                return self.extern_call(mname, ax, sp);
                            }
                            let mut known: Vec<String> = self
                                .funcs
                                .keys()
                                .filter(|(m, _)| *m == target)
                                .map(|(_, n)| n.split('.').next_back().unwrap_or(n).to_string())
                                .collect();
                            for s in self.short_types.get(&target).map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
                                known.push(s);
                            }
                            let hint = did_you_mean(mname, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                            self.diags.push(err(msp, format!("module '{}' has no function '{}'{}", target, mname, hint)));
                            return None;
                        }
                    }
                }
                let recv = self.infer(ctx, obj)?;
                let mut ax = Vec::new();
                for a in args {
                    ax.push(self.infer(ctx, a)?);
                }
                if let Ty::Named(qual) = resolve(&recv.ty) {
                    let si = self.prog.structs.get(&qual).cloned();
                    if let Some(si) = si {
                        if si.is_soa {
                            return self.soa_method(&qual, recv, mname, ax, msp);
                        }
                        let mkey = format!("{}.{}", si.name, mname);
                        if si.statics.iter().any(|m| *m == mkey) {
                            self.diags.push(err(msp, format!("'{}' is a static method of '{}' — call it as '{}'.'{}'(...)", mname, si.name, si.name, mname)));
                            return None;
                        }
                        if self.funcs.contains_key(&(si.module.clone(), mkey.clone())) {
                            let mut full = vec![recv];
                            full.extend(ax);
                            return self.call_fn(ctx, &si.module, &mkey, full, sp);
                        }
                        let mut known: Vec<String> = si.methods.iter().map(|m| m.split('.').next_back().unwrap_or(m).to_string()).collect();
                        known.extend(si.statics.iter().map(|m| m.split('.').next_back().unwrap_or(m).to_string()));
                        known.extend(si.fields.iter().map(|(f, _)| f.clone()));
                        let hint = did_you_mean(mname, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                        self.diags.push(err(msp, format!("type '{}' has no method '{}'{}", si.name, mname, hint)));
                        return None;
                    }
                }
                self.builtin_method(recv, mname, ax, msp)
            }
            other => {
                self.diags.push(err(&other.span(), "cannot call this expression".into()));
                None
            }
        }
    }

    /// Build an enum constructor call: `Shape.Circle(1.0, 2.0)`. Payload
    /// arguments are checked against the variant's payload types with the
    /// same assign/coercion rules as struct fields.
    fn enum_ctor(&mut self, eq: &str, variant: &str, ctx: &mut Ctx, args: &[Expr], sp: &Span, vsp: &Span) -> Option<Tx> {
        let (ename, vidx, pts) = {
            let ei = match self.prog.enums.get(eq) {
                Some(e) => e,
                None => return None,
            };
            match ei.variant_index(variant) {
                Some(i) => (ei.name.clone(), i, ei.variants[i].1.clone()),
                None => {
                    let known: Vec<String> = ei.variants.iter().map(|(n, _)| n.clone()).collect();
                    let hint = did_you_mean(variant, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                    self.diags.push(err(vsp, format!("enum '{}' has no variant '{}'{}", ei.name, variant, hint)));
                    return None;
                }
            }
        };
        if args.len() != pts.len() {
            let shape = if pts.is_empty() { variant.to_string() } else { format!("{}({})", variant, pts.iter().map(ty_name).collect::<Vec<_>>().join(", ")) };
            self.diags.push(err(
                sp,
                format!("variant '{}' takes {} payload value(s), got {} — write {}.{}(...)", variant, pts.len(), args.len(), ename, shape),
            ));
            return None;
        }
        let mut ax = Vec::new();
        for (a, pt) in args.iter().zip(pts.iter()) {
            let at = self.infer(ctx, a)?;
            if assign_ok(&at.ty, pt).is_none() && !coerce_holes(&at.ty, pt) {
                self.diags.push(err(&at.span, format!("payload of '{}' wants {}, got {}", variant, ty_name(pt), ty_name(&at.ty))));
                return None;
            }
            ax.push(at);
        }
        Some(Tx { ty: Ty::Named(eq.to_string()), span: sp.clone(), x: X::EnumLit(eq.to_string(), vidx, ax) })
    }

    fn extern_call(&mut self, name: &str, args: Vec<Tx>, sp: &Span) -> Option<Tx> {        let e = self.externs[name].clone();
        let n_fixed = e.params.len();
        if e.variadic {
            if args.len() < n_fixed {
                self.diags.push(err(sp, format!("extern '{}' takes at least {} argument(s), got {}", name, n_fixed, args.len())));
                return None;
            }
        } else if args.len() != n_fixed {
            self.diags.push(err(sp, format!("extern '{}' takes {} argument(s), got {}", name, n_fixed, args.len())));
            return None;
        }
        for (i, (_, a)) in e.params.iter().enumerate() {
            let want = match a {
                TypeAnnot::Int => "int",
                TypeAnnot::Float => "float",
                TypeAnnot::Bool => "bool",
                _ => "str",
            };
            let got = match resolve(&args[i].ty) {
                Ty::Int => "int",
                Ty::Float => "float",
                Ty::Bool => "bool",
                Ty::Str => "str",
                Ty::Never => continue,
                _ => {
                    self.diags.push(err(&args[i].span, format!("extern '{}' argument {} must be {}, got {}", name, i + 1, want, ty_name(&args[i].ty))));
                    return None;
                }
            };
            if got != want {
                self.diags.push(err(&args[i].span, format!("extern '{}' argument {} must be {}, got {}", name, i + 1, want, got)));
                return None;
            }
        }
        for a in &args[n_fixed.min(args.len())..] {
            match resolve(&a.ty) {
                Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Never => {}
                other => {
                    self.diags.push(err(&a.span, format!("extra arguments passed to a variadic extern must be int, float, bool or str, got {}", ty_name(&other))));
                    return None;
                }
            }
        }
        let ret = match &e.ret {
            Some(TypeAnnot::Int) => Ty::Int,
            Some(TypeAnnot::Float) => Ty::Float,
            Some(TypeAnnot::Bool) => Ty::Bool,
            Some(TypeAnnot::Str) => Ty::Str,
            _ => Ty::NoneTy,
        };
        Some(Tx { ty: ret, span: sp.clone(), x: X::CallExtern(name.to_string(), args) })
    }

    /// Resolve the callee name `SlotMap<int>` (parser-encoded in the ident
    /// string) to an instantiated type and build its constructor call.
    fn generic_ctor_call(&mut self, ctx: &mut Ctx, name: &str, ax: Vec<Tx>, sp: &Span) -> Option<Tx> {
        let a = match parse_annot_str(name) {
            Some(a) => a,
            None => {
                self.diags.push(err(sp, format!("malformed type arguments in '{}'", name)));
                return None;
            }
        };
        let (base, argannots) = match a {
            TypeAnnot::Generic(b, args) => (b, args),
            other => {
                self.diags.push(err(sp, format!("'{}' is not a generic type", annot_str(&other))));
                return None;
            }
        };
        let (tmod, _q, td) = match self.find_type(&ctx.module, &base) {
            Some(t) => t,
            None => {
                self.diags.push(err(sp, format!("unknown type '{}'", base)));
                return None;
            }
        };
        if td.params.is_empty() {
            self.diags.push(err(sp, format!("type '{}' is not generic; write {} instead of {}<...>", base, base, base)));
            return None;
        }
        let mut arg_tys = Vec::new();
        for sa in &argannots {
            match self.annot_ty_subst(&ctx.module, sa, &ctx.ty_params, &ctx.ty_args, sp) {
                Ok(t) => arg_tys.push(t),
                Err(m) => {
                    self.diags.push(err(sp, m));
                    return None;
                }
            }
        }
        let key = match self.instantiate_struct(&tmod, &base, &arg_tys, sp) {
            Some(k) => k,
            None => return None,
        };
        self.ctor(&key, ax, sp)
    }

    /// Inside a generic type's own method body, a bare constructor call of
    /// the owner type (`SlotMap()`) refers to the current instantiation.
    fn owner_ctor(&mut self, ctx: &Ctx, module: &str, name: &str, sp: &Span) -> Option<String> {
        let td = match self.find_type(module, name) {
            Some((tmod, _q, td)) if !td.params.is_empty() => (tmod, td),
            _ => return None,
        };
        let owner = ctx.owner_inst.as_ref()?;
        let si = self.prog.structs.get(owner)?;
        let base = si.name.split('<').next().unwrap_or(&si.name).to_string();
        if base == name && si.module == td.0 {
            Some(owner.clone())
        } else {
            self.diags.push(err(sp, format!("type '{}' takes {} type argument(s) — write {}<...> or call it inside one of its own methods", name, td.1.params.len(), name)));
            None
        }
    }

    fn ctor(&mut self, qual: &str, args: Vec<Tx>, sp: &Span) -> Option<Tx> {
        let si = self.prog.structs.get(qual).cloned();
        let si = match si {
            Some(s) => s,
            None => {
                // A bare generic type def has no StructInfo until
                // instantiated — that is an arity error, not an unknown type.
                if let Some(td) = self.types.get(qual) {
                    if !td.params.is_empty() {
                        self.diags.push(err(
                            sp,
                            format!(
                                "type '{}' takes {} type argument(s) — write {}<{}>",
                                td.name,
                                td.params.len(),
                                td.name,
                                td.params.join(", ")
                            ),
                        ));
                        return None;
                    }
                }
                self.diags.push(err(sp, format!("unknown type '{}'", qual)));
                return None;
            }
        };
        if si.is_soa {
            let cap = match args.len() {
                0 => Tx { ty: Ty::Int, span: sp.clone(), x: X::Int(0) },
                1 => match resolve(&args[0].ty) {
                    Ty::Int => args.into_iter().next().unwrap(),
                    other => {
                        self.diags.push(err(sp, format!("{}(capacity) needs an int capacity, got {}", si.name, ty_name(&other))));
                        return None;
                    }
                },
                n => {
                    self.diags.push(err(sp, format!("{}(capacity) takes 0 or 1 argument, got {}", si.name, n)));
                    return None;
                }
            };
            return Some(Tx { ty: Ty::Named(qual.to_string()), span: sp.clone(), x: X::SoaNew(qual.to_string(), Box::new(cap)) });
        }
        let n = si.fields.len();
        if args.len() > n {
            self.diags.push(err(sp, format!("{}(...) takes {} value(s), one per field, got {}", si.name, n, args.len())));
            return None;
        }
        let mut vals: Vec<(String, Tx)> = Vec::new();
        for i in 0..n {
            let (fname, fty) = si.fields[i].clone();
            if i < args.len() {
                if assign_ok(&args[i].ty, &fty).is_none() && !coerce_holes(&args[i].ty, &fty) {
                    self.diags.push(err(&args[i].span, format!("field '{}' of {} wants {}, got {}", fname, si.name, ty_name(&fty), ty_name(&args[i].ty))));
                    return None;
                }
                vals.push((fname, args[i].clone()));
            } else {
                match &si.defaults[i] {
                    Some(d) => vals.push((fname, d.clone())),
                    None => {
                        self.diags.push(err(sp, format!("missing value for field '{}' of {} (fields with no default must be given)", fname, si.name)));
                        return None;
                    }
                }
            }
        }
        Some(Tx { ty: Ty::Named(qual.to_string()), span: sp.clone(), x: X::StructLit(qual.to_string(), vals) })
    }

    fn call_fn(&mut self, _ctx: &mut Ctx, key_mod: &str, fname: &str, mut args: Vec<Tx>, sp: &Span) -> Option<Tx> {
        let (ptys, pdefs) = self.param_info(key_mod, fname);
        while args.len() < ptys.len() {
            match &pdefs[args.len()] {
                Some(d) => args.push(d.clone()),
                None => break,
            }
        }
        let (idx, ret) = self.instantiate(key_mod, fname, &args, sp, false)?;
        Some(Tx { ty: ret, span: sp.clone(), x: X::CallFn(idx, args) })
    }

    fn builtin_call(&mut self, _ctx: &mut Ctx, name: &str, args: Vec<Tx>, sp: &Span) -> BR {
        macro_rules! need {
            ($n:expr, $sig:expr) => {
                if args.len() != $n {
                    self.diags.push(err(sp, format!("{} takes {} argument(s), got {}", $sig, $n, args.len())));
                    return BR::Yes(None);
                }
            };
        }
        match name {
            "print" => BR::Yes(Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Builtin(Bi::Print, args) })),
            "len" => {
                need!(1, "len");
                match resolve(&args[0].ty) {
                    Ty::Str | Ty::List(_) | Ty::Dict(..) | Ty::Any => {}
                    Ty::Named(q) if self.prog.structs.get(&q).map(|s| s.is_soa).unwrap_or(false) => {}
                    other => {
                        self.diags.push(err(sp, format!("len works on str, list, dict and @soa values, not {}", ty_name(&other))));
                        return BR::Yes(None);
                    }
                }
                BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::Len, args) }))
            }
            "push" => {
                need!(2, "push");
                if let Ty::List(t) = resolve(&args[0].ty) {
                    if !self.compat(&args[1].ty, &t) {
                        self.diags.push(err(sp, format!("this list holds {}, cannot push {}", ty_name(&t), ty_name(&args[1].ty))));
                        return BR::Yes(None);
                    }
                    BR::Yes(Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Builtin(Bi::Push, args) }))
                } else {
                    self.diags.push(err(sp, "push takes a list and a value: push(items, item)".into()));
                    BR::Yes(None)
                }
            }
            "pop" => {
                need!(1, "pop");
                if let Ty::List(t) = resolve(&args[0].ty) {
                    BR::Yes(Some(Tx { ty: (*t).clone(), span: sp.clone(), x: X::Builtin(Bi::Pop, args) }))
                } else {
                    self.diags.push(err(sp, "pop takes a list: pop(items)".into()));
                    BR::Yes(None)
                }
            }
            "keys" => {
                need!(1, "keys");
                if let Ty::Dict(k, _) = resolve(&args[0].ty) {
                    BR::Yes(Some(Tx { ty: Ty::List(k), span: sp.clone(), x: X::Builtin(Bi::Keys, args) }))
                } else {
                    self.diags.push(err(sp, "keys takes a dict: keys(ages)".into()));
                    BR::Yes(None)
                }
            }
            "dict_key_at" => {
                need!(2, "dict_key_at");
                match resolve(&args[0].ty) {
                    Ty::Dict(..) | Ty::Any => {}
                    other => {
                        self.diags.push(err(sp, format!("dict_key_at works on dict and any values, not {}", ty_name(&other))));
                        return BR::Yes(None);
                    }
                }
                BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::DictKeyAt, args) }))
            }
            "read_dir" => {
                need!(1, "read_dir");
                match resolve(&args[0].ty) {
                    Ty::Str => BR::Yes(Some(Tx { ty: Ty::List(Rc::new(Ty::Str)), span: sp.clone(), x: X::Builtin(Bi::ReadDir, args) })),
                    other => {
                        self.diags.push(err(sp, format!("read_dir takes a path str, not {}", ty_name(&other))));
                        return BR::Yes(None);
                    }
                }
            }
            "file_exists" => {
                need!(1, "file_exists");
                match resolve(&args[0].ty) {
                    Ty::Str => BR::Yes(Some(Tx { ty: Ty::Bool, span: sp.clone(), x: X::Builtin(Bi::FileExists, args) })),
                    other => {
                        self.diags.push(err(sp, format!("file_exists takes a path str, not {}", ty_name(&other))));
                        return BR::Yes(None);
                    }
                }
            }
            "pairs" => {
                need!(1, "pairs");
                if let Ty::Dict(k, v) = resolve(&args[0].ty) {
                    let t = Ty::Tuple(Rc::new(vec![(*k).clone(), (*v).clone()]));
                    BR::Yes(Some(Tx { ty: Ty::List(Rc::new(t)), span: sp.clone(), x: X::Builtin(Bi::Pairs, args) }))
                } else {
                    self.diags.push(err(sp, "pairs takes a dict: pairs(ages)".into()));
                    BR::Yes(None)
                }
            }
            "str" => {
                need!(1, "str");
                BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::AsStr(Box::new(args.into_iter().next().unwrap())) }))
            }
            "bool" => {
                need!(1, "bool");
                BR::Yes(Some(Tx { ty: Ty::Bool, span: sp.clone(), x: X::Builtin(Bi::BoolOf, args) }))
            }
            "int" => {
                need!(1, "int");
                match resolve(&args[0].ty) {
                    Ty::Int | Ty::Float | Ty::Str | Ty::Bool | Ty::Never | Ty::Any => BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::IntOf, args) })),
                    other => {
                        self.diags.push(err(sp, format!("int() works on int, float, str and bool, not {}", ty_name(&other))));
                        BR::Yes(None)
                    }
                }
            }
            "float" => {
                need!(1, "float");
                match resolve(&args[0].ty) {
                    Ty::Int | Ty::Float | Ty::Str | Ty::Bool | Ty::Never | Ty::Any => BR::Yes(Some(Tx { ty: Ty::Float, span: sp.clone(), x: X::Builtin(Bi::FloatOf, args) })),
                    other => {
                        self.diags.push(err(sp, format!("float() works on int, float, str and bool, not {}", ty_name(&other))));
                        BR::Yes(None)
                    }
                }
            }
            "abs" => {
                need!(1, "abs");
                match resolve(&args[0].ty) {
                    Ty::Int | Ty::Float => BR::Yes(Some(Tx { ty: resolve(&args[0].ty), span: sp.clone(), x: X::Builtin(Bi::Abs, args) })),
                    _ => {
                        self.diags.push(err(sp, "abs works on numbers".into()));
                        BR::Yes(None)
                    }
                }
            }
            "min" | "max" => {
                let bi = if name == "min" { Bi::Min } else { Bi::Max };
                if args.len() == 1 {
                    if let Ty::List(t) = resolve(&args[0].ty) {
                        return BR::Yes(Some(Tx { ty: (*t).clone(), span: sp.clone(), x: X::Builtin(bi, args) }));
                    }
                    self.diags.push(err(sp, format!("{} takes several values, or one list", name)));
                    return BR::Yes(None);
                }
                if args.len() < 2 {
                    self.diags.push(err(sp, format!("{} needs at least 2 arguments", name)));
                    return BR::Yes(None);
                }
                let mut ty = args[0].ty.clone();
                for a in &args[1..] {
                    match self.unify_pair(&ty, &a.ty, sp, name) {
                        Some(t) => ty = t,
                        None => return BR::Yes(None),
                    }
                }
                BR::Yes(Some(Tx { ty, span: sp.clone(), x: X::Builtin(bi, args) }))
            }
            "sum" => {
                need!(1, "sum");
                if let Ty::List(t) = resolve(&args[0].ty) {
                    match resolve(&t) {
                        Ty::Int | Ty::Float | Ty::Var(_) => BR::Yes(Some(Tx { ty: (*t).clone(), span: sp.clone(), x: X::Builtin(Bi::Sum, args) })),
                        other => {
                            self.diags.push(err(sp, format!("sum works on lists of numbers, not [{}]", ty_name(&other))));
                            BR::Yes(None)
                        }
                    }
                } else {
                    self.diags.push(err(sp, "sum takes a list: sum(nums)".into()));
                    BR::Yes(None)
                }
            }
            "range" => match args.len() {
                1 | 2 | 3 => {
                    for a in &args {
                        match resolve(&a.ty) {
                            Ty::Int | Ty::Never => {}
                            Ty::Var(cell) => {
                                *cell.borrow_mut() = Some(Ty::Int);
                            }
                            other => {
                                self.diags.push(err(&a.span, format!("range values must be int, not {}", ty_name(&other))));
                                return BR::Yes(None);
                            }
                        }
                    }
                    let bi = match args.len() {
                        1 => Bi::Range,
                        2 => Bi::Range2,
                        _ => Bi::Range3,
                    };
                    BR::Yes(Some(Tx { ty: Ty::List(Rc::new(Ty::Int)), span: sp.clone(), x: X::Builtin(bi, args) }))
                }
                n => {
                    self.diags.push(err(sp, format!("range takes 1 to 3 arguments, got {}", n)));
                    BR::Yes(None)
                }
            },
            "enumerate" => {
                need!(1, "enumerate");
                if let Ty::List(t) = resolve(&args[0].ty) {
                    let pair = Ty::Tuple(Rc::new(vec![Ty::Int, (*t).clone()]));
                    BR::Yes(Some(Tx { ty: Ty::List(Rc::new(pair)), span: sp.clone(), x: X::Builtin(Bi::Enumerate, args) }))
                } else {
                    self.diags.push(err(sp, "enumerate works on lists (for i, x in enumerate(items))".into()));
                    BR::Yes(None)
                }
            }
            "reversed" => {
                need!(1, "reversed");
                if let Ty::List(t) = resolve(&args[0].ty) {
                    BR::Yes(Some(Tx { ty: Ty::List(t), span: sp.clone(), x: X::Builtin(Bi::Reversed, args) }))
                } else {
                    self.diags.push(err(sp, "reversed takes a list".into()));
                    BR::Yes(None)
                }
            }
            "type_of" => {
                need!(1, "type_of");
                BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::TypeOf, args) }))
            }
            "input" => match args.len() {
                0 => BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::Input, args) })),
                1 => {
                    if !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) {
                        self.diags.push(err(sp, "the input prompt must be a str".into()));
                        return BR::Yes(None);
                    }
                    BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::InputPrompt, args) }))
                }
                n => {
                    self.diags.push(err(sp, format!("input takes 0 or 1 argument, got {}", n)));
                    BR::Yes(None)
                }
            },
            "clock" => {
                need!(0, "clock");
                BR::Yes(Some(Tx { ty: Ty::Float, span: sp.clone(), x: X::Builtin(Bi::Clock, args) }))
            }
            "rand" => match args.len() {
                0 => BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::Rand, args) })),
                1 => {
                    if !matches!(resolve(&args[0].ty), Ty::Int | Ty::Never) {
                        self.diags.push(err(sp, "rand(bound) needs an int".into()));
                        return BR::Yes(None);
                    }
                    BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::RandBound, args) }))
                }
                n => {
                    self.diags.push(err(sp, format!("rand takes 0 or 1 argument, got {}", n)));
                    BR::Yes(None)
                }
            },
            "seed" => {
                need!(1, "seed");
                if !matches!(resolve(&args[0].ty), Ty::Int | Ty::Never) {
                    self.diags.push(err(sp, "seed takes an int".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Builtin(Bi::Seed, args) }))
            }
            "check" => {
                if args.is_empty() || args.len() > 2 {
                    self.diags.push(err(sp, "check takes a condition and an optional message".into()));
                    return BR::Yes(None);
                }
                if args.len() == 2 && !matches!(resolve(&args[1].ty), Ty::Str | Ty::Never) {
                    self.diags.push(err(sp, "the check message must be a str".into()));
                    return BR::Yes(None);
                }
                let bi = if args.len() == 1 { Bi::Check } else { Bi::CheckMsg };
                BR::Yes(Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Builtin(bi, args) }))
            }
            "fail" => {
                need!(1, "fail");
                if !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) {
                    self.diags.push(err(sp, "fail takes a str message".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::Never, span: sp.clone(), x: X::Builtin(Bi::Fail, args) }))
            }
            "exit" => {
                need!(1, "exit");
                if !matches!(resolve(&args[0].ty), Ty::Int | Ty::Never) {
                    self.diags.push(err(sp, "exit takes an int code".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::Never, span: sp.clone(), x: X::Builtin(Bi::Exit, args) }))
            }
            "ord" => {
                need!(1, "ord");
                if !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) {
                    self.diags.push(err(sp, "ord takes a one-character str".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::Ord, args) }))
            }
            "chr" => {
                need!(1, "chr");
                if !matches!(resolve(&args[0].ty), Ty::Int | Ty::Never) {
                    self.diags.push(err(sp, "chr takes an int".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::Chr, args) }))
            }
            "read_file" => {
                need!(1, "read_file");
                BR::Yes(Some(Tx { ty: Ty::Str, span: sp.clone(), x: X::Builtin(Bi::ReadFile, args) }))
            }
            "write_file" => {
                need!(2, "write_file");
                BR::Yes(Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Builtin(Bi::WriteFile, args) }))
            }
            "get_env" => {
                need!(1, "get_env");
                BR::Yes(Some(Tx { ty: Ty::Nullable(Box::new(Ty::Str)), span: sp.clone(), x: X::Builtin(Bi::GetEnv, args) }))
            }
            "args" => {
                need!(0, "args");
                BR::Yes(Some(Tx { ty: Ty::List(Rc::new(Ty::Str)), span: sp.clone(), x: X::Builtin(Bi::Args, args) }))
            }
            "sleep_ms" => {
                need!(1, "sleep_ms");
                if !matches!(resolve(&args[0].ty), Ty::Int | Ty::Never) {
                    self.diags.push(err(sp, "sleep_ms takes an int".into()));
                    return BR::Yes(None);
                }
                BR::Yes(Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Builtin(Bi::SleepMs, args) }))
            }
            _ => BR::No,
        }
    }

    fn builtin_method(&mut self, recv: Tx, name: &str, args: Vec<Tx>, sp: &Span) -> Option<Tx> {
        let rt = resolve(&recv.ty);
        let m = |x: X, ty: Ty| Some(Tx { ty, span: sp.clone(), x });
        match &rt {
            Ty::List(elem) => {
                let elem = (**elem).clone();
                match name {
                    "push" => {
                        if args.len() != 1 {
                            self.diags.push(err(sp, format!("push takes 1 value, got {}", args.len())));
                            return None;
                        }
                        if !self.compat(&args[0].ty, &elem) {
                            self.diags.push(err(sp, format!("this list holds {}, cannot push {}", ty_name(&elem), ty_name(&args[0].ty))));
                            return None;
                        }
                        m(X::Method(Box::new(recv), "push".into(), args), Ty::NoneTy)
                    }
                    "pop" => {
                        if !args.is_empty() {
                            self.diags.push(err(sp, "pop takes no arguments".into()));
                            return None;
                        }
                        m(X::Method(Box::new(recv), "pop".into(), args), elem)
                    }
                    "len" => {
                        if !args.is_empty() {
                            self.diags.push(err(sp, "len takes no arguments".into()));
                            return None;
                        }
                        m(X::Method(Box::new(recv), "len".into(), args), Ty::Int)
                    }
                    "clear" => {
                        if !args.is_empty() {
                            self.diags.push(err(sp, "clear takes no arguments".into()));
                            return None;
                        }
                        m(X::Method(Box::new(recv), "clear".into(), args), Ty::NoneTy)
                    }
                    other => {
                        self.diags.push(err(sp, format!("lists have no method '{}' (they have push, pop, len, clear)", other)));
                        None
                    }
                }
            }
            Ty::Str => match name {
                "len" => m(X::Method(Box::new(recv), "len".into(), args), Ty::Int),
                "upper" | "lower" | "trim" => m(X::Method(Box::new(recv), name.into(), args), Ty::Str),
                "contains" | "starts" | "ends" => {
                    if args.len() != 1 || !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) {
                        self.diags.push(err(sp, format!("{} takes one str", name)));
                        return None;
                    }
                    m(X::Method(Box::new(recv), name.into(), args), Ty::Bool)
                }
                "split" => {
                    if args.len() != 1 || !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) {
                        self.diags.push(err(sp, "split takes one str separator".into()));
                        return None;
                    }
                    m(X::Method(Box::new(recv), "split".into(), args), Ty::List(Rc::new(Ty::Str)))
                }
                "join" => {
                    let want: Ty = Ty::List(Rc::new(Ty::Str));
                    if args.len() != 1 || (assign_ok(&args[0].ty, &want).is_none() && !coerce_holes(&args[0].ty, &want)) {
                        self.diags.push(err(sp, "join takes one list of str to combine".into()));
                        return None;
                    }
                    m(X::Method(Box::new(recv), "join".into(), args), Ty::Str)
                }
                "replace" => {
                    if args.len() != 2 || !matches!(resolve(&args[0].ty), Ty::Str | Ty::Never) || !matches!(resolve(&args[1].ty), Ty::Str | Ty::Never) {
                        self.diags.push(err(sp, "replace takes two str arguments (from, to)".into()));
                        return None;
                    }
                    m(X::Method(Box::new(recv), "replace".into(), args), Ty::Str)
                }
                other => {
                    self.diags.push(err(sp, format!("strings have no method '{}' (they have len, upper, lower, trim, contains, starts, ends, split, join, replace)", other)));
                    None
                }
            },
            Ty::Dict(k, v) => match name {
                "len" => m(X::Method(Box::new(recv), "len".into(), args), Ty::Int),
                "clear" => m(X::Method(Box::new(recv), "clear".into(), args), Ty::NoneTy),
                "get" => {
                    if args.len() != 2 {
                        self.diags.push(err(sp, format!("get takes (key, default), got {}", args.len())));
                        return None;
                    }
                    let kt = (**k).clone();
                    let vt = (**v).clone();
                    if assign_ok(&args[0].ty, &kt).is_none() && !coerce_holes(&args[0].ty, &kt) {
                        self.diags.push(err(sp, format!("dict key must be {}", ty_name(&kt))));
                        return None;
                    }
                    if assign_ok(&args[1].ty, &vt).is_none() && !coerce_holes(&args[1].ty, &vt) {
                        self.diags.push(err(sp, format!("the default value must be {}", ty_name(&vt))));
                        return None;
                    }
                    m(X::Method(Box::new(recv), "get".into(), args), vt)
                }
                "pop" => {
                    if args.len() != 1 {
                        self.diags.push(err(sp, format!("pop takes (key), got {}", args.len())));
                        return None;
                    }
                    let kt = (**k).clone();
                    let vt = (**v).clone();
                    if assign_ok(&args[0].ty, &kt).is_none() && !coerce_holes(&args[0].ty, &kt) {
                        self.diags.push(err(sp, format!("dict key must be {}", ty_name(&kt))));
                        return None;
                    }
                    m(X::Method(Box::new(recv), "pop".into(), args), vt)
                }
                other => {
                    self.diags.push(err(sp, format!("dicts have no method '{}' (they have len, clear, get, pop; try keys(d) and 'in')", other)));
                    None
                }
            },
            Ty::Named(q) => {
                let si = self.prog.structs.get(q).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        return self.soa_method(q, recv, name, args, sp);
                    }
                }
                self.diags.push(err(sp, format!("{} has no method '{}'", ty_name(&rt), name)));
                None
            }
            other => {
                self.diags.push(err(sp, format!("{} has no methods", ty_name(other))));
                None
            }
        }
    }

    fn soa_method(&mut self, qual: &str, recv: Tx, name: &str, mut args: Vec<Tx>, sp: &Span) -> Option<Tx> {
        let si = self.prog.structs.get(qual).cloned()?;
        match name {
            "push" => {
                if args.len() != si.fields.len() {
                    self.diags.push(err(sp, format!("push on {} takes {} value(s), one per field, got {}", si.name, si.fields.len(), args.len())));
                    return None;
                }
                for (i, (fname, fty)) in si.fields.iter().enumerate() {
                    if assign_ok(&args[i].ty, fty).is_none() && !coerce_holes(&args[i].ty, fty) {
                        self.diags.push(err(&args[i].span, format!("field '{}' wants {}, got {}", fname, ty_name(fty), ty_name(&args[i].ty))));
                        return None;
                    }
                }
                args.insert(0, recv);
                Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Method(Box::new(args.remove(0)), "soa_push".into(), args) })
            }
            "clear" => Some(Tx { ty: Ty::NoneTy, span: sp.clone(), x: X::Method(Box::new(recv), "soa_clear".into(), args) }),
            "len" => Some(Tx { ty: Ty::Int, span: sp.clone(), x: X::Method(Box::new(recv), "soa_len".into(), args) }),
            other => {
                self.diags.push(err(sp, format!("@soa values have no method '{}' (they have push, clear, len)", other)));
                None
            }
        }
    }
}

fn assign_ok(src: &Ty, dst: &Ty) -> Option<&'static str> {
    let s = resolve(src);
    let d = resolve(dst);
    if ty_eq(&s, &d) {
        return Some("same");
    }
    match (&s, &d) {
        (Ty::Int, Ty::Float) => Some("widen"),
        (Ty::NoneTy, Ty::Nullable(_)) => Some("null"),
        (Ty::NoneTy, Ty::NoneTy) => Some("same"),
        (_, Ty::Any) if boxable(&s) => Some("boxany"),
        (_, Ty::Nullable(inner)) if ty_eq(&s, inner) => Some("box"),
        _ => None,
    }
}

/// Can a value of this type be stored in an `any`? Scalars, str, none and
/// (recursively) lists/dicts whose elements are boxable. Unbound element
/// holes are pinned to `any` so `a: any = []` works. Structs, tuples and
/// nullables cannot go into an `any`.
fn boxable(t: &Ty) -> bool {
    match resolve(t) {
        Ty::List(e) => boxable(&e),
        Ty::Dict(k, v) => boxable(&k) && boxable(&v),
        Ty::Var(cell) => match &*cell.borrow() {
            Some(inner) => boxable(inner),
            None => {
                *cell.borrow_mut() = Some(Ty::Any);
                true
            }
        },
        other => any_box_tag(&other).is_some(),
    }
}

/// Fill an inference hole: a bare empty container literal carries an unbound
/// type variable for its element type. Adopt `want` into it when it is anything
/// concrete. Returns true when the source now fits the destination.
fn bind_hole(hole: &Rc<Ty>, want: &Rc<Ty>) -> bool {
    let cell = match &**hole {
        Ty::Var(cell) => cell.clone(),
        _ => return false,
    };
    let free = cell.borrow().is_none();
    if !free {
        return false;
    }
    let w = resolve(want);
    match w {
        Ty::Var(_) => false,
        w => {
            *cell.borrow_mut() = Some(w);
            true
        }
    }
}

/// When a check fails structurally, let bare `[]` / `{}` literals (whose
/// element/key/value types are still unbound variables) adopt the destination
/// container's types, then report success.
fn coerce_holes(src: &Ty, dst: &Ty) -> bool {
    let s = resolve(src);
    let d = resolve(dst);
    match (&s, &d) {
        (Ty::List(a), Ty::List(b)) => bind_hole(a, b) || bind_hole(b, a),
        (Ty::Dict(ak, av), Ty::Dict(bk, bv)) => {
            (bind_hole(ak, bk) || bind_hole(bk, ak)) && (bind_hole(av, bv) || bind_hole(bv, av))
        }
        (Ty::Tuple(xs), Ty::Tuple(ws)) => {
            xs.len() == ws.len()
                && xs.iter().zip(ws.iter()).all(|(x, w)| match (resolve(x), resolve(w)) {
                    (Ty::List(a), Ty::List(b)) => bind_hole(&a, &b),
                    _ => false,
                })
        }
        _ => false,
    }
}

fn mangle_cname(module: &str, fname: &str, tys: &[Ty], generic: bool, attrs: &[String]) -> String {
    if attrs.iter().any(|a| a == "export") {
        return sanitize(&fname.replace('.', "_"));
    }
    let base = format!("still_{}__{}", sanitize(module), sanitize(&fname.replace('.', "_")));
    if generic {
        format!("{}__{}", base, tys.iter().map(ty_tag).collect::<Vec<_>>().join("_"))
    } else {
        base
    }
}

/// True when `e` is an empty list or dict literal (their type is fully
/// determined by the destination).
fn is_empty_literal(e: &Expr) -> bool {
    match e {
        Expr::ListLit(v, _) => v.is_empty(),
        Expr::DictLit(v, _) => v.is_empty(),
        _ => false,
    }
}

/// Empty container literal of a concrete type (used to coerce empty-literal
/// field defaults to the instantiated field type).
fn empty_literal_x(ty: &Ty) -> X {
    match resolve(ty) {
        Ty::Dict(..) => X::DictLit(Vec::new()),
        _ => X::ListLit(Vec::new()),
    }
}

fn has_break(stmts: &[CS]) -> bool {    for s in stmts {
        match s {
            CS::Break(_) => return true,
            CS::If { then, els, .. } => {
                if has_break(then) || has_break(els) {
                    return true;
                }
            }
            CS::While { body, .. } => {
                if has_break(body) {
                    return true;
                }
            }
            CS::ForList { body, .. }
            | CS::ForRange { body, .. }
            | CS::ForEnum { body, .. }
            | CS::ForStr { body, .. }
            | CS::ForSoa { body, .. } => {
                if has_break(body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn terminates(stmts: &[CS]) -> bool {
    let Some(last) = stmts.last() else { return false };
    match last {
        CS::Return { .. } => true,
        CS::ExprS(t) => matches!(resolve(&t.ty), Ty::Never),
        CS::If { then, els, .. } => !els.is_empty() && terminates(then) && terminates(els),
        CS::While { cond, body } => matches!(&cond.x, X::Bool(true)) && !has_break(body),
        _ => false,
    }
}

impl Checker {
    fn unify_ret(&mut self, ctx: &mut Ctx, t: Ty, sp: &Span) {
        if matches!(resolve(&t), Ty::Never) {
            return;
        }
        let cell = ctx.ret.clone();
        let cur = match &*cell.borrow() {
            Some(x) => resolve(x),
            None => Ty::Var(cell.clone()),
        };
        if let Ty::Var(c2) = &cur {
            if std::rc::Rc::ptr_eq(c2, &cell) {
                *cell.borrow_mut() = Some(resolve(&t));
                return;
            }
        }
        let new = resolve(&t);
        if ctx.ret_annotated {
            if assign_ok(&new, &cur).is_none() && !coerce_holes(&new, &cur) {
                self.diags.push(err(sp, format!("this returns {} but the function is declared to return {}", ty_name(&new), ty_name(&cur))));
            }
            return;
        }
        if ty_eq(&cur, &new) {
            return;
        }
        if let Ty::Var(c) = &cur {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(new);
                return;
            }
        }
        if let Ty::Var(c) = &new {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(cur);
                return;
            }
        }
        match (&cur, &new) {
            (Ty::NoneTy, o) | (o, Ty::NoneTy) => {
                *cell.borrow_mut() = Some(Ty::Nullable(Box::new(o.clone())));
            }
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => {
                *cell.borrow_mut() = Some(Ty::Float);
            }
            (Ty::Nullable(a), b) if ty_eq(a, b) => {}
            (a, Ty::Nullable(b)) if ty_eq(a, b) => {
                *cell.borrow_mut() = Some(Ty::Nullable(Box::new(a.clone())));
            }
            _ => {
                self.diags.push(err(sp, format!("this function returns {} in one place and {} in another; make them match or annotate the return type", ty_name(&cur), ty_name(&new))));
            }
        }
    }

    fn instantiate(&mut self, key_mod: &str, fname: &str, args: &[Tx], sp: &Span, eager: bool) -> Option<(usize, Ty)> {
        let fd = match self.funcs.get(&(key_mod.to_string(), fname.to_string())) {
            Some(f) => f.clone(),
            None => {
                self.diags.push(err(sp, format!("internal: unknown function '{}'", fname)));
                return None;
            }
        };
        let (ptys, pdefs) = self.param_info(key_mod, fname);
        let np = ptys.len();
        if !eager {
            if args.len() > np {
                self.diags.push(err(sp, format!("'{}' takes {} argument(s), got {}", fname, np, args.len())));
                return None;
            }
            if args.len() < np {
                let fillable = pdefs.len() == np && pdefs[args.len()..].iter().all(|d| d.is_some());
                if !fillable {
                    self.diags.push(err(sp, format!("'{}' takes {} argument(s), got {}", fname, np, args.len())));
                    return None;
                }
            }
        }
        let mut key_tys: Vec<Ty> = Vec::new();
        for i in 0..np {
            match &ptys[i] {
                Some(t) => {
                    if !eager {
                        let arg_ty = if i < args.len() { args[i].ty.clone() } else { pdefs[i].as_ref().unwrap().ty.clone() };
                        if assign_ok(&arg_ty, t).is_none() && !coerce_holes(&arg_ty, t) {
                            self.diags.push(err(sp, format!("argument {} of '{}' ('{}') wants {}, got {}", i + 1, fname, fd.params[i].name, ty_name(t), ty_name(&arg_ty))));
                            return None;
                        }
                    }
                    key_tys.push(t.clone());
                }
                None => {
                    if fd.params[i].name == "self" {
                        if i < args.len() {
                            key_tys.push(resolve(&args[i].ty));
                        } else {
                            let tname = fname.split('.').next().unwrap();
                            // type_qual_of covers plain types (short_types)
                            // and instantiated generic types (inst_by_name);
                            // the eager stdlib typecheck calls methods with
                            // no receiver, so self comes from the name.
                            let qual = self.type_qual_of(key_mod, tname);
                            key_tys.push(qual.map(Ty::Named).unwrap_or(Ty::NoneTy));
                        }
                    } else if eager {
                        self.diags.push(err(sp, format!("internal: eager check of generic '{}'", fname)));
                        return None;
                    } else if i < args.len() {
                        let t = resolve(&args[i].ty);
                        if let Ty::Var(cell) = &t {
                            let _ = cell;
                            self.diags.push(err(&args[i].span, format!("cannot infer the type of argument {} ('{}'); annotate the value or the parameter", i + 1, fd.params[i].name)));
                            return None;
                        }
                        if matches!(t, Ty::NoneTy) {
                            self.diags.push(err(&args[i].span, format!("cannot infer the type of parameter '{}' from none", fd.params[i].name)));
                            return None;
                        }
                        key_tys.push(t);
                    } else {
                        self.diags.push(err(sp, format!("'{}' takes {} argument(s), got {}", fname, np, args.len())));
                        return None;
                    }
                }
            }
        }
        let generic = fd.params.iter().enumerate().any(|(i, p)| p.name != "self" && ptys[i].is_none());
        let tags: Vec<String> = key_tys.iter().map(ty_tag).collect();
        let key = format!("{}::{}|{}", key_mod, fname, tags.join(","));
        if let Some(idx) = self.insts.get(&key) {
            let ret = self.prog.funcs[*idx].ret.clone();
            return Some((*idx, ret));
        }
        if generic && fd.attrs.iter().any(|a| a == "export") {
            self.diags.push(err(sp, format!("generic functions cannot be @export (they become one copy per type used)", )));
            return None;
        }
        let ret_cell: VarCell = Rc::new(std::cell::RefCell::new(None));
        let ret = match &fd.ret {
            Some(a) => match self.annot_ty_in(key_mod, a, &fd.span) {
                Ok(t) => t,
                Err(m) => {
                    self.diags.push(err(&fd.span, m));
                    Ty::NoneTy
                }
            },
            None => Ty::Var(ret_cell.clone()),
        };
        // A method is only self-by-pointer when it actually takes a `self`
        // param first; statics (Player.make()) share the dotted name but have
        // no receiver. Owner lookup covers instantiated generic types too
        // ("SlotMap<i>" lives in inst_by_name, not short_types).
        let is_method = fname.contains('.') && !fd.params.is_empty() && fd.params[0].name == "self";
        let self_ty = if is_method {
            let tname = fname.split('.').next().unwrap();
            self.type_qual_of(key_mod, tname)
        } else {
            None
        };
        let params: Vec<(String, Ty)> = fd
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.clone(), resolve_deep(&key_tys[i])))
            .collect();
        let cname = mangle_cname(key_mod, fname, &key_tys, generic, &fd.attrs);
        let idx = self.prog.funcs.len();
        self.prog.funcs.push(CFunc {
            cname,
            source_name: fname.to_string(),
            module: key_mod.to_string(),
            params,
            ret: ret.clone(),
            body: Vec::new(),
            exported: fd.attrs.iter().any(|a| a == "export"),
            self_ty,
        });
        self.insts.insert(key, idx);
        let mut ctx = Ctx::new(key_mod.to_string(), true);
        ctx.self_param = is_method;
        // Inside a per-instantiation generic method body, bare constructor
        // calls to the owner type resolve to this instantiation, and explicit
        // `SlotMap<T>`-style args substitute T.
        if fname.contains('.') {
            let tname = fname.split('.').next().unwrap();
            if let Some(q) = self.inst_by_name.get(&(key_mod.to_string(), tname.to_string())).cloned() {
                if let Some(si) = self.prog.structs.get(&q) {
                    let base_name = tname.split('<').next().unwrap().to_string();
                    let base_qual = format!("{}::{}", key_mod, base_name);
                    if let Some(td) = self.types.get(&base_qual) {
                        ctx.owner_inst = Some(q.clone());
                        ctx.ty_params = td.params.clone();
                        ctx.ty_args = si.ty_args.clone();
                    }
                }
            }
        }
        if fd.ret.is_some() {
            *ctx.ret.borrow_mut() = Some(ret.clone());
            ctx.ret_annotated = true;
        } else {
            ctx.ret = ret_cell;
        }
        for (i, p) in fd.params.iter().enumerate() {
            ctx.declare(&p.name, resolve_deep(&key_tys[i]));
        }
        let mut body = Vec::new();
        for s in &fd.body {
            let _ = self.check_stmt(&mut ctx, s, &mut body);
        }
        let final_ty = resolve(&Ty::Var(ctx.ret.clone()));
        if let Ty::Var(c) = &final_ty {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(Ty::NoneTy);
            }
        }
        let final_resolved = resolve(&self.prog.funcs[idx].ret);
        if !matches!(final_resolved, Ty::NoneTy | Ty::Never) && !terminates(&body) {
            self.diags.push(err(&fd.span, format!("'{}' can reach the end without returning a value; add a return or make every if/else path return", fname)));
        }
        self.prog.funcs[idx].body = body;
        let r = self.prog.funcs[idx].ret.clone();
        Some((idx, r))
    }

    fn refinements(&self, ctx: &Ctx, cond: &Expr) -> (HashSet<String>, HashSet<String>) {
        let nullable_local = |x: &str| -> bool {
            match ctx.lookup(x) {
                Some(id) => matches!(resolve(&ctx.locals[id as usize].1), Ty::Nullable(_)),
                None => false,
            }
        };
        match cond {
            Expr::Ident(x, _) => {
                if nullable_local(x) {
                    let mut s = HashSet::new();
                    s.insert(x.clone());
                    (s, HashSet::new())
                } else {
                    (HashSet::new(), HashSet::new())
                }
            }
            Expr::IsNone(inner, neg, _) => {
                if let Expr::Ident(x, _) = inner.as_ref() {
                    if nullable_local(x) {
                        let mut a = HashSet::new();
                        let mut b = HashSet::new();
                        if *neg {
                            a.insert(x.clone());
                        } else {
                            b.insert(x.clone());
                        }
                        return (a, b);
                    }
                }
                (HashSet::new(), HashSet::new())
            }
            Expr::Not(inner, _) => {
                if let Expr::IsNone(i2, false, _) = inner.as_ref() {
                    if let Expr::Ident(x, _) = i2.as_ref() {
                        if nullable_local(x) {
                            let mut s = HashSet::new();
                            s.insert(x.clone());
                            return (s, HashSet::new());
                        }
                    }
                }
                (HashSet::new(), HashSet::new())
            }
            Expr::And(a, b, _) => {
                let (mut ta, _) = self.refinements(ctx, a);
                let (tb, _) = self.refinements(ctx, b);
                ta.extend(tb);
                (ta, HashSet::new())
            }
            _ => (HashSet::new(), HashSet::new()),
        }
    }

    fn check_stmt(&mut self, ctx: &mut Ctx, s: &Stmt, out: &mut Vec<CS>) -> Option<()> {
        match s {
            Stmt::Pass(_) => out.push(CS::Pass),
            Stmt::Global(names, sp) => {
                if !ctx.in_fn {
                    return Some(()); // module-level `global` is a no-op (Python rule)
                }
                for name in names {
                    if ctx.lookup(name).is_some() {
                        self.diags.push(err(sp, format!("'{}' is assigned to before its 'global' declaration", name)));
                    }
                    if !self.gmap.contains_key(&(ctx.module.clone(), name.clone())) {
                        self.diags.push(err(sp, format!("global '{}' must exist at module level; assign it at the top level first", name)));
                    }
                    ctx.global_names.insert(name.clone());
                }
            }
            Stmt::Break(sp) => {
                if ctx.loops == 0 {
                    self.diags.push(err(sp, "break can only be used inside a loop".into()));
                    return None;
                }
                out.push(CS::Break(sp.clone()));
            }
            Stmt::Continue(sp) => {
                if ctx.loops == 0 {
                    self.diags.push(err(sp, "continue can only be used inside a loop".into()));
                    return None;
                }
                out.push(CS::Continue(sp.clone()));
            }
            Stmt::Return { value, span } => {
                if !ctx.in_fn {
                    self.diags.push(err(span, "return can only be used inside a function".into()));
                    return None;
                }
                let tx = match value {
                    Some(e) => self.infer(ctx, e)?,
                    None => Tx { ty: Ty::NoneTy, span: span.clone(), x: X::NoneV },
                };
                self.unify_ret(ctx, tx.ty.clone(), span);
                out.push(CS::Return { value: Some(tx), span: span.clone() });
            }
            Stmt::ExprStmt { expr, .. } => {
                let tx = self.infer(ctx, expr)?;
                out.push(CS::ExprS(tx));
            }
            Stmt::If { arms, else_body, .. } => {
                let cs = self.check_if(ctx, arms, else_body)?;
                out.push(cs);
            }
            Stmt::Match { subject, arms, span } => {
                self.check_match(ctx, subject, arms, span, out)?;
            }
            Stmt::While { cond, body, .. } => {
                let ct = self.infer(ctx, cond)?;
                ctx.scopes.push(HashMap::new());
                ctx.loops += 1;
                let mut b = Vec::new();
                for st in body {
                    let _ = self.check_stmt(ctx, st, &mut b);
                }
                ctx.loops -= 1;
                ctx.scopes.pop();
                out.push(CS::While { cond: ct, body: b });
            }
            Stmt::For { vars, iter, body, span } => {
                self.check_for(ctx, vars, iter, body, span, out)?;
            }
            Stmt::Assign { targets, op, value, annot, span } => {
                self.check_assign(ctx, targets, *op, annot, value, span, out)?;
            }
            _ => {}
        }
        Some(())
    }

    fn check_if(&mut self, ctx: &mut Ctx, arms: &[(Expr, Vec<Stmt>)], else_body: &Option<Vec<Stmt>>) -> Option<CS> {
        let mut infos: Vec<(Tx, Vec<CS>, HashSet<String>)> = Vec::new();
        for (cond, body) in arms {
            let ct = self.infer(ctx, cond)?;
            let (th, el) = self.refinements(ctx, cond);
            ctx.scopes.push(HashMap::new());
            for n in &th {
                ctx.refined.insert(n.clone());
            }
            let mut b = Vec::new();
            for st in body {
                let _ = self.check_stmt(ctx, st, &mut b);
            }
            for n in &th {
                ctx.refined.remove(n);
            }
            ctx.scopes.pop();
            infos.push((ct, b, el));
        }
        let mut els: Option<Vec<CS>> = None;
        if let Some(eb) = else_body {
            let el = infos.last().map(|(_, _, e)| e.clone()).unwrap_or_default();
            ctx.scopes.push(HashMap::new());
            for n in &el {
                ctx.refined.insert(n.clone());
            }
            let mut b = Vec::new();
            for st in eb {
                let _ = self.check_stmt(ctx, st, &mut b);
            }
            for n in &el {
                ctx.refined.remove(n);
            }
            ctx.scopes.pop();
            els = Some(b);
        }
        let mut cur: Vec<CS> = els.unwrap_or_default();
        for (ct, body, _) in infos.into_iter().rev() {
            cur = vec![CS::If { cond: ct, then: body, els: cur }];
        }
        cur.pop()
    }

    /// Validate a match statement and lower it to an equivalent if/elif
    /// chain. Scalar literal arms lower to `subject in [values]`, reusing
    /// check_if and the existing X::In emit path. Tuple/list patterns —
    /// binding or all-literal — lower to a tower of nested ifs (length and
    /// element tests, outermost first) whose innermost body binds the
    /// pattern's names and then runs the arm body, so a failed inner test
    /// falls through to the next arm and binders reuse the ordinary Assign
    /// machinery as arm-locals. A `_` arm becomes the else branch; with no
    /// `_` arm, non-matching subjects fall through (Python rule).
    fn check_match(
        &mut self,
        ctx: &mut Ctx,
        subject: &Expr,
        arms: &[MatchCase],
        span: &Span,
        out: &mut Vec<CS>,
    ) -> Option<()> {
        // Restrict the subject to a plain variable: each lowered condition
        // re-evaluates the subject, and a call subject would run once per arm.
        if !matches!(subject, Expr::Ident(_, _)) {
            self.diags.push(err(span, "match subject must be a plain variable".into()));
            return None;
        }
        let subj_name = match subject {
            Expr::Ident(n, _) => n.clone(),
            _ => String::new(),
        };
        let sty = self.infer(ctx, subject)?;
        let sres = resolve(&sty.ty);
        // An enum subject enables variant patterns; other named types are
        // rejected with the regular subject message.
        let subj_enum: Option<(String, EnumInfo)> = match &sres {
            Ty::Named(q) => match self.prog.enums.get(q).cloned() {
                Some(ei) => Some((q.clone(), ei)),
                None => {
                    self.diags.push(err(span, format!("match subject must be int, str, any, tuple, list or an enum, not {}", ty_name(&sty.ty))));
                    return None;
                }
            },
            Ty::Int | Ty::Str | Ty::Any | Ty::Tuple(_) | Ty::List(_) => None,
            _ => {
                self.diags.push(err(span, format!("match subject must be int, str, any, tuple, list or an enum, not {}", ty_name(&sty.ty))));
                return None;
            }
        };
        if !matches!(sres, Ty::Int | Ty::Str | Ty::Any | Ty::Tuple(_) | Ty::List(_) | Ty::Named(_)) {
            self.diags.push(err(span, format!("match subject must be int, str, any, tuple, list or an enum, not {}", ty_name(&sty.ty))));
            return None;
        }
        enum ArmLower {
            /// Scalar literal arm: one `subject in [values]` condition.
            Value { cond: Expr, body: Vec<Stmt>, span: Span },
            /// Tuple/list pattern arm: length/element tests whose innermost
            /// body binds the pattern's names and runs the arm body.
            Tower { conds: Vec<Expr>, body: Vec<Stmt>, span: Span },
        }
        let mut lowered_arms: Vec<ArmLower> = Vec::new();
        let mut else_body: Option<Vec<Stmt>> = None;
        let mut seen: HashSet<String> = HashSet::new();
        let mut wildcard_seen = false;
        let mut binder_names: HashSet<String> = HashSet::new();
        for arm in arms {
            if wildcard_seen {
                self.diags.push(err(&arm.span, "'_' must be the last case arm".into()));
                return None;
            }
            if arm.wildcard {
                wildcard_seen = true;
                else_body = Some(arm.body.clone());
                continue;
            }
            if let Some((eq, ei)) = &subj_enum {
                // Enum subject: each arm is exactly one variant pattern.
                if arm.values.len() != 1 {
                    self.diags.push(err(&arm.span, "an enum case arm must contain exactly one variant pattern".into()));
                    return None;
                }
                let (vidx, vsp) = self.enum_arm_variant(&ei, &arm.values[0])?;
                if !seen.insert(format!("e:{}:{}", eq, vidx)) {
                    self.diags.push(err(&arm.span, "duplicate case variant".into()));
                    return None;
                }
                let npts = ei.variants[vidx].1.len();
                let vn = ei.variants[vidx].0.clone();
                let items = self.enum_arm_payloads(&arm.values[0], &vn, npts, &vsp)?;
                let (conds, body, names) = self.lower_enum_arm(ctx, &subj_name, eq, vidx, &items, arm)?;
                binder_names.extend(names);
                lowered_arms.push(ArmLower::Tower { conds, body, span: arm.span.clone() });
                continue;
            }
            let mut value_pats: Vec<Expr> = Vec::new();
            let mut binding: Option<&Expr> = None;
            for v in &arm.values {
                match v {
                    Expr::IntLit(n, vsp) => {
                        self.pat_value_gate(&sres, true, vsp)?;
                        if !seen.insert(format!("i:{}", n)) {
                            self.diags.push(err(vsp, "duplicate case value".into()));
                            return None;
                        }
                        value_pats.push(v.clone());
                    }
                    Expr::StrLit(s, vsp) => {
                        self.pat_value_gate(&sres, false, vsp)?;
                        if !seen.insert(format!("s:{}", s)) {
                            self.diags.push(err(vsp, "duplicate case value".into()));
                            return None;
                        }
                        value_pats.push(v.clone());
                    }
                    Expr::Tuple(..) | Expr::ListLit(..) => {
                        // Tuple/list patterns always go through the binding
                        // tower, literals or not: the runtime has no
                        // list-of-container representation, so the `in`
                        // lowering can't compare whole containers, and a `_`
                        // mixed with literals must not leak into it either.
                        if binding.is_some() || !value_pats.is_empty() {
                            self.diags.push(err(&arm.span, "a case arm with a binding pattern must contain exactly one pattern".into()));
                            return None;
                        }
                        binding = Some(v);
                    }
                    _ => {
                        self.diags.push(err(&v.span(), "case patterns must be int or str literals, or (…) / […] patterns".into()));
                        return None;
                    }
                }
            }
            if binding.is_some() && !value_pats.is_empty() {
                self.diags.push(err(&arm.span, "a case arm with a binding pattern must contain exactly one pattern".into()));
                return None;
            }
            if let Some(pat) = binding {
                let is_tuple = matches!(pat, Expr::Tuple(..));
                let (conds, body, names) = self.lower_binding_arm(ctx, &subj_name, &sres, pat, is_tuple, arm)?;
                binder_names.extend(names);
                lowered_arms.push(ArmLower::Tower { conds, body, span: arm.span.clone() });
            } else {
                let list = Expr::ListLit(value_pats, arm.span.clone());
                let cond = Expr::In(Box::new(subject.clone()), Box::new(list), false, arm.span.clone());
                lowered_arms.push(ArmLower::Value { cond, body: arm.body.clone(), span: arm.span.clone() });
            }
        }
        // Fold the arms into one nested if-chain, last arm first: every
        // arm carries the remaining chain in the else branch of each of
        // its tests, so a failed test at any level (scalar cond or tower
        // level) falls through to the next arm and finally to `_`.
        let mut cur: Vec<Stmt> = else_body.clone().unwrap_or_default();
        for arm in lowered_arms.into_iter().rev() {
            match arm {
                ArmLower::Value { cond, body, span } => {
                    cur = vec![Stmt::If { arms: vec![(cond, body)], else_body: Some(cur), span }];
                }
                ArmLower::Tower { conds, body, span } => {
                    if conds.is_empty() {
                        // Unconditional arm (e.g. a tuple pattern with only
                        // binders): it always matches, so the remaining
                        // arms are unreachable. It still runs inside its
                        // own branch scope so its binders stay arm-local.
                        cur = vec![Stmt::If { arms: vec![(Expr::BoolLit(true, span.clone()), body)], else_body: None, span }];
                        continue;
                    }
                    let mut node: Option<Stmt> = None;
                    for c in conds.into_iter().rev() {
                        let then_branch = match node.take() {
                            Some(prev) => vec![prev],
                            None => body.clone(),
                        };
                        node = Some(Stmt::If { arms: vec![(c, then_branch)], else_body: Some(cur.clone()), span: span.clone() });
                    }
                    if let Some(n) = node {
                        cur = vec![n];
                    }
                }
            }
        }
        // The binder set is active only while the lowered arms are checked;
        // a nested match swaps in its own set and restores this one.
        let saved_binders = std::mem::replace(&mut ctx.pat_binders, binder_names);
        let mut checked = Some(());
        for st in &cur {
            if self.check_stmt(ctx, st, out).is_none() {
                checked = None;
                break;
            }
        }
        ctx.pat_binders = saved_binders;
        checked
    }

    /// Gate an int/str case value against the subject type.
    fn pat_value_gate(&mut self, sres: &Ty, is_int: bool, sp: &Span) -> Option<()> {
        match sres {
            Ty::Int if is_int => Some(()),
            Ty::Str if !is_int => Some(()),
            Ty::Any => Some(()),
            Ty::Int => {
                self.diags.push(err(sp, "str case value cannot match an int subject".into()));
                None
            }
            Ty::Str => {
                self.diags.push(err(sp, "int case value cannot match a str subject".into()));
                None
            }
            other => {
                self.diags.push(err(sp, format!("int/str case values cannot match a {} subject", ty_name(other))));
                None
            }
        }
    }

    /// Lower one binding arm: validate the subject shape and walk the
    /// pattern into test conditions (outermost first) and binder
    /// assignments. The caller wraps the conditions into the nested-if
    /// tower whose innermost body is `body` (binders, then the arm body).
    /// Binder names are declared as arm-locals by check_assign's
    /// pattern-binder path, never as module globals.
    fn lower_binding_arm(
        &mut self,
        ctx: &mut Ctx,
        subj_name: &str,
        sres: &Ty,
        pat: &Expr,
        is_tuple: bool,
        arm: &MatchCase,
    ) -> Option<(Vec<Expr>, Vec<Stmt>, Vec<String>)> {
        let items = match pat {
            Expr::Tuple(xs, _) | Expr::ListLit(xs, _) => xs,
            _ => return None,
        };
        if is_tuple {
            if !matches!(sres, Ty::Tuple(_)) {
                self.diags.push(err(&arm.span, format!("tuple pattern needs a tuple subject, not {}", ty_name(sres))));
                return None;
            }
        } else {
            if !matches!(sres, Ty::List(_)) {
                self.diags.push(err(&arm.span, format!("list pattern needs a list subject, not {}", ty_name(sres))));
                return None;
            }
            // The lowered arity test calls len(); it must be the built-in.
            let shadowed = ctx.lookup("len").is_some()
                || self.funcs.contains_key(&(ctx.module.clone(), "len".to_string()))
                || self.imports.get(&ctx.module).map(|m| m.contains_key("len")).unwrap_or(false)
                || self.gmap.contains_key(&(ctx.module.clone(), "len".to_string()));
            if shadowed {
                self.diags.push(err(&arm.span, "list patterns need the built-in len; rename your own 'len'".into()));
                return None;
            }
        }
        let path = Expr::Ident(subj_name.to_string(), arm.span.clone());
        let mut conds: Vec<Expr> = Vec::new();
        let mut binds: Vec<Stmt> = Vec::new();
        self.pat_walk(&path, sres, items, &arm.span, &mut conds, &mut binds)?;
        let names: Vec<String> = binds.iter().filter_map(|s| match s {
            Stmt::Assign { targets, .. } if targets.len() == 1 => match &targets[0] {
                Expr::Ident(n, _) => Some(n.clone()),
                _ => None,
            },
            _ => None,
        }).collect();
        let mut body = binds;
        body.extend(arm.body.iter().cloned());
        Some((conds, body, names))
    }

    /// Walk one pattern level against its container type, appending tests to
    /// `conds` (outermost first, so a later test is only reached after every
    /// earlier one passed) and binder assignments to `binds`.
    fn pat_walk(&mut self, path: &Expr, cty: &Ty, items: &[Expr], sp: &Span, conds: &mut Vec<Expr>, binds: &mut Vec<Stmt>) -> Option<()> {
        match cty {
            Ty::Tuple(ts) => {
                if ts.len() != items.len() {
                    self.diags.push(err(sp, format!("tuple pattern has {} elements but the subject tuple has {}", items.len(), ts.len())));
                    return None;
                }
                for (i, it) in items.iter().enumerate() {
                    let esp = it.span();
                    let child = Expr::Index(Box::new(path.clone()), Box::new(Expr::IntLit(i as i64, esp.clone())), esp);
                    self.pat_elem(&child, &resolve_deep(&ts[i]), it, sp, conds, binds)?;
                }
            }
            Ty::List(elem) => {
                // The runtime arity test comes first: every later test and
                // binder indexes the subject and is only safe once it passed.
                conds.push(Expr::Cmp(
                    Box::new(Expr::Call(
                        Box::new(Expr::Ident("len".into(), sp.clone())),
                        vec![path.clone()],
                        sp.clone(),
                    )),
                    CmpOp::Eq,
                    Box::new(Expr::IntLit(items.len() as i64, sp.clone())),
                    sp.clone(),
                ));
                let et = resolve_deep(elem);
                for (i, it) in items.iter().enumerate() {
                    let esp = it.span();
                    let child = Expr::Index(Box::new(path.clone()), Box::new(Expr::IntLit(i as i64, esp.clone())), esp);
                    self.pat_elem(&child, &et, it, sp, conds, binds)?;
                }
            }
            _ => {
                self.diags.push(err(sp, format!("pattern needs a tuple or list subject, not {}", ty_name(cty))));
                return None;
            }
        }
        Some(())
    }

    fn pat_elem(&mut self, child: &Expr, cty: &Ty, it: &Expr, sp: &Span, conds: &mut Vec<Expr>, binds: &mut Vec<Stmt>) -> Option<()> {
        match it {
            Expr::Ident(n, nsp) => {
                if n == "_" {
                    return Some(());
                }
                binds.push(Stmt::Assign {
                    targets: vec![Expr::Ident(n.clone(), nsp.clone())],
                    op: None,
                    value: child.clone(),
                    annot: None,
                    span: nsp.clone(),
                });
                Some(())
            }
            Expr::Tuple(sub, _) | Expr::ListLit(sub, _) => {
                self.pat_walk(child, cty, sub, sp, conds, binds)
            }
            Expr::IntLit(..) | Expr::FloatLit(..) | Expr::BoolLit(..) | Expr::StrLit(..) => {
                conds.push(Expr::Cmp(Box::new(child.clone()), CmpOp::Eq, Box::new(it.clone()), sp.clone()));
                Some(())
            }
            Expr::Member(base, vname, msp) => {
                // Qualified zero-payload nested variant: `Shape.Point`.
                let ename = match base.as_ref() {
                    Expr::Ident(e, _) => e.clone(),
                    _ => {
                        self.diags.push(err(msp, "invalid nested variant pattern".into()));
                        return None;
                    }
                };
                let eq = match resolve(cty) {
                    Ty::Named(q) if self.prog.enums.contains_key(&q) => q,
                    other => {
                        self.diags.push(err(msp, format!("this pattern only matches enum values, not {}", ty_name(&other))));
                        return None;
                    }
                };
                let ei = self.prog.enums[&eq].clone();
                if ename != ei.name {
                    self.diags.push(err(msp, format!("'{}' is not the enum type here (expected '{}')", ename, ei.name)));
                    return None;
                }
                let vidx = match ei.variant_index(vname) {
                    Some(i) => i,
                    None => {
                        let known: Vec<String> = ei.variants.iter().map(|(n, _)| n.clone()).collect();
                        let hint = did_you_mean(vname, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                        self.diags.push(err(msp, format!("enum '{}' has no variant '{}'{}", ei.name, vname, hint)));
                        return None;
                    }
                };
                self.pat_variant_walk(child, &eq, vidx, &[], sp, conds, binds)
            }
            Expr::Call(callee, items, csp) => {
                // Nested variant pattern: `case Circle(Inner(x, y))` or
                // `case Circle(Shape.Inner(x))` — the payload type must be an
                // enum and the callee one of its variants.
                let eq = match resolve(cty) {
                    Ty::Named(q) if self.prog.enums.contains_key(&q) => q,
                    other => {
                        self.diags.push(err(csp, format!("this pattern only matches enum values, not {}", ty_name(&other))));
                        return None;
                    }
                };
                let (vname, ename) = match callee.as_ref() {
                    Expr::Ident(n, _) => (n.clone(), None),
                    Expr::Member(b, n, _) => {
                        let e = match b.as_ref() {
                            Expr::Ident(e, _) => e.clone(),
                            _ => {
                                self.diags.push(err(&callee.span(), "invalid nested variant pattern".into()));
                                return None;
                            }
                        };
                        (n.clone(), Some(e))
                    }
                    other => {
                        self.diags.push(err(&other.span(), "invalid nested variant pattern".into()));
                        return None;
                    }
                };
                let ei = self.prog.enums[&eq].clone();
                if let Some(e) = ename {
                    if e != ei.name {
                        self.diags.push(err(csp, format!("'{}' is not the enum type here (expected '{}')", e, ei.name)));
                        return None;
                    }
                }
                let vidx = match ei.variant_index(&vname) {
                    Some(i) => i,
                    None => {
                        let known: Vec<String> = ei.variants.iter().map(|(n, _)| n.clone()).collect();
                        let hint = did_you_mean(&vname, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                        self.diags.push(err(csp, format!("enum '{}' has no variant '{}'{}", ei.name, vname, hint)));
                        return None;
                    }
                };
                self.pat_variant_walk(child, &eq, vidx, items, sp, conds, binds)
            }
            _ => {
                self.diags.push(err(&it.span(), "unsupported pattern element".into()));
                None
            }
        }
    }

    /// Walk one enum variant pattern against `path` (an expression denoting
    /// the enum value): a tag test, then per-payload sub-patterns through
    /// pat_elem against the flat payload fields.
    fn pat_variant_walk(&mut self, path: &Expr, eq: &str, vidx: usize, items: &[Expr], sp: &Span, conds: &mut Vec<Expr>, binds: &mut Vec<Stmt>) -> Option<()> {
        let (vname, pts) = {
            let ei = &self.prog.enums[eq];
            let (n, p) = &ei.variants[vidx];
            (n.clone(), p.clone())
        };
        conds.push(Expr::Cmp(
            Box::new(Expr::Member(Box::new(path.clone()), "tag".to_string(), sp.clone())),
            CmpOp::Eq,
            Box::new(Expr::IntLit(vidx as i64, sp.clone())),
            sp.clone(),
        ));
        if items.len() != pts.len() {
            self.diags.push(err(
                sp,
                format!(
                    "variant '{}' takes {} payload pattern(s), got {} — write {}({})",
                    vname,
                    pts.len(),
                    items.len(),
                    vname,
                    vec!["p0"; pts.len()].join(", ")
                ),
            ));
            return None;
        }
        for (i, it) in items.iter().enumerate() {
            let esp = it.span();
            let fname = enum_payload_field(&vname, i);
            let child = Expr::Member(Box::new(path.clone()), fname, esp.clone());
            self.pat_elem(&child, &resolve_deep(&pts[i]), it, sp, conds, binds)?;
        }
        Some(())
    }

    /// Resolve a top-level enum case arm pattern to its variant index and
    /// span. Accepts `Circle`, `Shape.Circle`, `Circle(...)` and
    /// `Shape.Circle(...)` shapes (the parser encodes all four).
    fn enum_arm_variant(&mut self, ei: &EnumInfo, pat: &Expr) -> Option<(usize, Span)> {
        let (vname, vsp) = match pat {
            Expr::Ident(n, s) => (n.clone(), s.clone()),
            Expr::Call(callee, _, _) => match callee.as_ref() {
                Expr::Ident(n, s) => (n.clone(), s.clone()),
                Expr::Member(_, n, s) => (n.clone(), s.clone()),
                other => {
                    self.diags.push(err(&other.span(), "invalid variant pattern".into()));
                    return None;
                }
            },
            Expr::Member(_, n, s) => (n.clone(), s.clone()),
            other => {
                self.diags.push(err(&other.span(), "enum case patterns must be variant names or '_'".into()));
                return None;
            }
        };
        match ei.variant_index(&vname) {
            Some(i) => Some((i, vsp)),
            None => {
                let known: Vec<String> = ei.variants.iter().map(|(n, _)| n.clone()).collect();
                let hint = did_you_mean(&vname, &known).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                self.diags.push(err(&vsp, format!("enum '{}' has no variant '{}'{}", ei.name, vname, hint)));
                None
            }
        }
    }

    /// Extract the payload sub-patterns of a top-level enum case arm,
    /// rejecting a bare variant name when the variant has payloads.
    fn enum_arm_payloads(&mut self, pat: &Expr, vname: &str, npts: usize, arm_sp: &Span) -> Option<Vec<Expr>> {
        match pat {
            Expr::Ident(..) => {
                if npts > 0 {
                    let ph: Vec<String> = (0..npts).map(|i| format!("p{}", i)).collect();
                    self.diags.push(err(arm_sp, format!("variant '{}' has payloads; write '{}({})'", vname, vname, ph.join(", "))));
                    return None;
                }
                Some(Vec::new())
            }
            Expr::Call(_, items, _) => Some(items.clone()),
            Expr::Member(_, _, _) => {
                if npts > 0 {
                    let ph: Vec<String> = (0..npts).map(|i| format!("p{}", i)).collect();
                    self.diags.push(err(arm_sp, format!("variant '{}' has payloads; write '{}({})'", vname, vname, ph.join(", "))));
                } else {
                    self.diags.push(err(arm_sp, format!("variant '{}' takes no payloads; write '{}'", vname, vname)));
                }
                None
            }
            _ => {
                self.diags.push(err(arm_sp, "invalid variant pattern".into()));
                None
            }
        }
    }

    /// Lower one enum case arm: tag test plus payload sub-pattern walk. The
    /// arm body runs after the binders inside the innermost branch.
    fn lower_enum_arm(
        &mut self,
        ctx: &mut Ctx,
        subj_name: &str,
        eq: &str,
        vidx: usize,
        items: &[Expr],
        arm: &MatchCase,
    ) -> Option<(Vec<Expr>, Vec<Stmt>, Vec<String>)> {
        let _ = ctx;
        let path = Expr::Ident(subj_name.to_string(), arm.span.clone());
        let mut conds: Vec<Expr> = Vec::new();
        let mut binds: Vec<Stmt> = Vec::new();
        self.pat_variant_walk(&path, eq, vidx, items, &arm.span, &mut conds, &mut binds)?;
        let names: Vec<String> = binds.iter().filter_map(|s| match s {
            Stmt::Assign { targets, .. } if targets.len() == 1 => match &targets[0] {
                Expr::Ident(n, _) => Some(n.clone()),
                _ => None,
            },
            _ => None,
        }).collect();
        let mut body = binds;
        body.extend(arm.body.iter().cloned());
        Some((conds, body, names))
    }

    fn check_for(&mut self, ctx: &mut Ctx, vars: &[String], iter: &Expr, body: &[Stmt], span: &crate::span::Span, out: &mut Vec<CS>) -> Option<()> {
        let module = ctx.module.clone();
        if let Expr::Call(callee, args, _) = iter {
            if let Expr::Ident(cname, _) = callee.as_ref() {
                let shadowed = ctx.lookup(cname).is_some()
                    || self.funcs.contains_key(&(module.clone(), cname.clone()))
                    || self.imports.get(&module).map(|m| m.contains_key(cname)).unwrap_or(false)
                    || self.gmap.contains_key(&(module.clone(), cname.clone()));
                if !shadowed && cname == "range" && vars.len() == 1 && (1..=3).contains(&args.len()) {
                    let mut ax = Vec::new();
                    for a in args {
                        ax.push(self.want_int(ctx, a, "range value")?);
                    }
                    let zero = Tx { ty: Ty::Int, span: span.clone(), x: X::Int(0) };
                    let one = Tx { ty: Ty::Int, span: span.clone(), x: X::Int(1) };
                    let (start, end, step) = match ax.len() {
                        1 => (zero, ax.pop().unwrap(), one),
                        2 => {
                            let e = ax.pop().unwrap();
                            let s = ax.pop().unwrap();
                            (s, e, one)
                        }
                        _ => {
                            let st = ax.pop().unwrap();
                            let e = ax.pop().unwrap();
                            let s = ax.pop().unwrap();
                            (s, e, st)
                        }
                    };
                    if let X::Int(0) = &step.x {
                        self.diags.push(err(span, "range step cannot be 0".into()));
                        return None;
                    }
                    let var_name = vars[0].clone();
                    ctx.scopes.push(HashMap::new());
                    let id = ctx.declare(&var_name, Ty::Int);
                    ctx.loop_vars.insert(var_name);
                    ctx.loops += 1;
                    let mut b = Vec::new();
                    for st in body {
                        let _ = self.check_stmt(ctx, st, &mut b);
                    }
                    ctx.loops -= 1;
                    ctx.loop_vars.remove(&vars[0]);
                    ctx.scopes.pop();
                    out.push(CS::ForRange { start, end, step, var: id, body: b });
                    return Some(());
                }
                if !shadowed && cname == "enumerate" && vars.len() == 2 && args.len() == 1 {
                    let it = self.infer(ctx, &args[0])?;
                    if let Ty::List(elem) = resolve(&it.ty) {
                        let elem = (*elem).clone();
                        ctx.scopes.push(HashMap::new());
                        let iv = ctx.declare(&vars[0], Ty::Int);
                        let ev = ctx.declare(&vars[1], elem);
                        ctx.loop_vars.insert(vars[0].clone());
                        ctx.loop_vars.insert(vars[1].clone());
                        ctx.loops += 1;
                        let mut b = Vec::new();
                        for st in body {
                            let _ = self.check_stmt(ctx, st, &mut b);
                        }
                        ctx.loops -= 1;
                        ctx.loop_vars.remove(&vars[0]);
                        ctx.loop_vars.remove(&vars[1]);
                        ctx.scopes.pop();
                        out.push(CS::ForEnum { list: it, ivar: iv, evar: ev, body: b });
                        return Some(());
                    }
                    self.diags.push(err(span, "enumerate works on a list, with two loop variables: for i, x in enumerate(items)".into()));
                    return None;
                }
            }
        }
        let it = self.infer(ctx, iter)?;
        match resolve(&it.ty) {
            Ty::List(elem) => {
                if vars.len() == 2 {
                    let et = resolve_deep(&elem);
                    if let Ty::Tuple(ts) = &et {
                        if ts.len() == 2 {
                            let (ta, tb) = (ts[0].clone(), ts[1].clone());
                            ctx.scopes.push(HashMap::new());
                            let av = ctx.declare(&vars[0], ta);
                            let bv = ctx.declare(&vars[1], tb);
                            ctx.loop_vars.insert(vars[0].clone());
                            ctx.loop_vars.insert(vars[1].clone());
                            ctx.loops += 1;
                            let mut b = Vec::new();
                            for st in body {
                                let _ = self.check_stmt(ctx, st, &mut b);
                            }
                            ctx.loops -= 1;
                            ctx.loop_vars.remove(&vars[0]);
                            ctx.loop_vars.remove(&vars[1]);
                            ctx.scopes.pop();
                            out.push(CS::ForPairs { list: it, avar: av, bvar: bv, body: b });
                            return Some(());
                        }
                    }
                }
                if vars.len() != 1 {
                    self.diags.push(err(span, "a list needs one loop variable (use enumerate for the index too)".into()));
                    return None;
                }
                let elem = (*elem).clone();
                ctx.scopes.push(HashMap::new());
                let id = ctx.declare(&vars[0], elem);
                ctx.loop_vars.insert(vars[0].clone());
                ctx.loops += 1;
                let mut b = Vec::new();
                for st in body {
                    let _ = self.check_stmt(ctx, st, &mut b);
                }
                ctx.loops -= 1;
                ctx.loop_vars.remove(&vars[0]);
                ctx.scopes.pop();
                out.push(CS::ForList { list: it, var: id, body: b });
                Some(())
            }
            Ty::Str => {
                if vars.len() != 1 {
                    self.diags.push(err(span, "a str needs one loop variable".into()));
                    return None;
                }
                ctx.scopes.push(HashMap::new());
                let id = ctx.declare(&vars[0], Ty::Str);
                ctx.loop_vars.insert(vars[0].clone());
                ctx.loops += 1;
                let mut b = Vec::new();
                for st in body {
                    let _ = self.check_stmt(ctx, st, &mut b);
                }
                ctx.loops -= 1;
                ctx.loop_vars.remove(&vars[0]);
                ctx.scopes.pop();
                out.push(CS::ForStr { s: it, var: id, body: b });
                Some(())
            }
            Ty::Dict(k, _) => {
                if vars.len() != 1 {
                    self.diags.push(err(span, "a dict needs one loop variable (it gets the keys)".into()));
                    return None;
                }
                let kt = (*k).clone();
                let keys_tx = Tx { ty: Ty::List(Rc::new(kt.clone())), span: span.clone(), x: X::Builtin(Bi::Keys, vec![it]) };
                ctx.scopes.push(HashMap::new());
                let id = ctx.declare(&vars[0], kt);
                ctx.loop_vars.insert(vars[0].clone());
                ctx.loops += 1;
                let mut b = Vec::new();
                for st in body {
                    let _ = self.check_stmt(ctx, st, &mut b);
                }
                ctx.loops -= 1;
                ctx.loop_vars.remove(&vars[0]);
                ctx.scopes.pop();
                out.push(CS::ForList { list: keys_tx, var: id, body: b });
                Some(())
            }
            Ty::Any => {
                if vars.len() > 2 {
                    self.diags.push(err(span, "looping over an 'any' value takes one variable (the item), or two for (index, item)".into()));
                    return None;
                }
                ctx.scopes.push(HashMap::new());
                let iv = if vars.len() == 2 { Some(ctx.declare(&vars[0], Ty::Int)) } else { None };
                let ev = ctx.declare(&vars[vars.len() - 1], Ty::Any);
                for v in vars {
                    ctx.loop_vars.insert(v.clone());
                }
                ctx.loops += 1;
                let mut b = Vec::new();
                for st in body {
                    let _ = self.check_stmt(ctx, st, &mut b);
                }
                ctx.loops -= 1;
                for v in vars {
                    ctx.loop_vars.remove(v);
                }
                ctx.scopes.pop();
                out.push(CS::ForAny { list: it, ivar: iv, var: ev, body: b });
                Some(())
            }
            Ty::Named(qual) => {
                let si = self.prog.structs.get(&qual).cloned();
                if let Some(si) = si {
                    if si.is_soa {
                        if vars.len() != 1 {
                            self.diags.push(err(span, "looping a @soa value gives indexes; use one variable".into()));
                            return None;
                        }
                        ctx.scopes.push(HashMap::new());
                        let id = ctx.declare(&vars[0], Ty::Int);
                        ctx.loop_vars.insert(vars[0].clone());
                        ctx.loops += 1;
                        let mut b = Vec::new();
                        for st in body {
                            let _ = self.check_stmt(ctx, st, &mut b);
                        }
                        ctx.loops -= 1;
                        ctx.loop_vars.remove(&vars[0]);
                        ctx.scopes.pop();
                        out.push(CS::ForSoa { soa: it, var: id, body: b });
                        return Some(());
                    }
                }
                self.diags.push(err(span, format!("cannot loop over a single {}", ty_name(&Ty::Named(qual)))));
                None
            }
            other => {
                self.diags.push(err(span, format!("cannot loop over {}; use range(n) for numbers, or a list, str, dict or @soa value", ty_name(&other))));
                None
            }
        }
    }

    fn check_assign(&mut self, ctx: &mut Ctx, targets: &[Expr], op: Option<BinOp>, annot: &Option<TypeAnnot>, value: &Expr, span: &crate::span::Span, out: &mut Vec<CS>) -> Option<()> {
        if let Some(op) = op {
            let (place, pty) = self.check_target(ctx, &targets[0])?;
            let vt = self.infer(ctx, value)?;
            let dummy = Tx { ty: pty.clone(), span: span.clone(), x: X::NoneV };
            let res = self.type_arith(ctx, op, dummy, vt.clone(), span)?;
            if !ty_eq(&res.ty, &pty) {
                let hint = if op == BinOp::Div {
                    " (x / y gives a float; use // to keep whole numbers, or declare the variable as float)"
                } else {
                    ""
                };
                self.diags.push(err(span, format!("this operation produces {} but the target holds {}{}", ty_name(&res.ty), ty_name(&pty), hint)));
                return None;
            }
            out.push(CS::AugAssign { place, op, value: vt, ty: pty, span: span.clone() });
            return Some(());
        }
        if targets.len() == 1 {
            if let Expr::Ident(name, nsp) = &targets[0] {
                let vt = self.infer(ctx, value)?;
                let module = ctx.module.clone();
                if let Some(id) = ctx.lookup(name) {
                    if annot.is_some() {
                        self.diags.push(err(nsp, format!("'{}' already has a type; annotations only go on the first assignment", name)));
                        return None;
                    }
                    if ctx.loop_vars.contains(name) {
                        self.diags.push(err(nsp, format!("cannot assign to the loop variable '{}'", name)));
                        return None;
                    }
                    if ctx.self_param && id == 0 {
                        self.diags.push(err(nsp, "cannot assign to self (it is a copy of the value; build a new one and return it)".into()));
                        return None;
                    }
                    let cur = ctx.locals[id as usize].1.clone();
                    // `s = s + t` on a str local folds into the in-place append
                    // so loops stay linear instead of quadratic.
                    if matches!(resolve(&cur), Ty::Str) {
                        if let Expr::Binary(BinOp::Add, lhs, _, _) = value {
                            if let Expr::Ident(iname, _) = lhs.as_ref() {
                                if iname == name {
                                    if let X::Arith(_, _, rtx) = &vt.x {
                                        if matches!(resolve(&rtx.ty), Ty::Str) {
                                            out.push(CS::AugAssign { place: Place::Local(id), op: BinOp::Add, value: (**rtx).clone(), ty: cur, span: span.clone() });
                                            return Some(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if matches!(resolve(&cur), Ty::NoneTy) {
                        if !matches!(resolve(&vt.ty), Ty::NoneTy | Ty::Never) {
                            if let Ty::Var(cell) = &cur {
                                *cell.borrow_mut() = Some(Ty::Nullable(Box::new(resolve(&vt.ty))));
                                let newty = ctx.locals[id as usize].1.clone();
                                out.push(CS::Assign { place: Place::Local(id), value: vt, ty: newty, span: span.clone() });
                                return Some(());
                            }
                        }
                    }
                    if assign_ok(&vt.ty, &cur).is_none() && !coerce_holes(&vt.ty, &cur) {
                        let hint = if matches!(resolve(&vt.ty), Ty::NoneTy) {
                            format!(" (to allow none, declare it as {}: {}? = none)", name, ty_name(&cur))
                        } else {
                            String::new()
                        };
                        self.diags.push(err(&vt.span, format!("cannot assign {} to '{}' which holds {}{}", ty_name(&vt.ty), name, ty_name(&cur), hint)));
                        return None;
                    }
                    out.push(CS::Assign { place: Place::Local(id), value: vt, ty: cur, span: span.clone() });
                    return Some(());
                }
                if let Some(cname) = self.gmap.get(&(module.clone(), name.clone())).cloned() {
                    if ctx.in_fn && !ctx.global_names.contains(name) {
                        let id = ctx.declare(name, vt.ty.clone());
                        out.push(CS::Decl { local: id, name: name.clone(), ty: vt.ty.clone(), value: Some(vt) });
                        return Some(());
                    }
                    let cur = self.prog.globals[&cname].ty.clone();
                    // same `s = s + t` fold for globals
                    if matches!(resolve(&cur), Ty::Str) {
                        if let Expr::Binary(BinOp::Add, lhs, _, _) = value {
                            if let Expr::Ident(iname, _) = lhs.as_ref() {
                                if iname == name {
                                    if let X::Arith(_, _, rtx) = &vt.x {
                                        if matches!(resolve(&rtx.ty), Ty::Str) {
                                            out.push(CS::AugAssign { place: Place::Global(cname.clone()), op: BinOp::Add, value: (**rtx).clone(), ty: cur, span: span.clone() });
                                            return Some(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if matches!(resolve(&cur), Ty::NoneTy) {
                        if !matches!(resolve(&vt.ty), Ty::NoneTy | Ty::Never) {
                            if let Ty::Var(cell) = &cur {
                                *cell.borrow_mut() = Some(Ty::Nullable(Box::new(resolve(&vt.ty))));
                                let newty = self.prog.globals[&cname].ty.clone();
                                out.push(CS::Assign { place: Place::Global(cname), value: vt, ty: newty, span: span.clone() });
                                return Some(());
                            }
                        }
                    }
                    if assign_ok(&vt.ty, &cur).is_none() && !coerce_holes(&vt.ty, &cur) {
                        self.diags.push(err(&vt.span, format!("cannot assign {} to '{}' which holds {}", ty_name(&vt.ty), name, ty_name(&cur))));
                        return None;
                    }
                    out.push(CS::Assign { place: Place::Global(cname), value: vt, ty: cur, span: span.clone() });
                    return Some(());
                }
                if let Some(a) = annot {
                    let aty = match self.annot_ty_in(&ctx.module, a, nsp) {
                        Ok(t) => t,
                        Err(m) => {
                            self.diags.push(err(nsp, m));
                            return None;
                        }
                    };
                    let fits = self.deep_compat(&vt.ty, &aty) || assign_ok(&vt.ty, &aty).is_some() || coerce_holes(&vt.ty, &aty);
                    if !matches!(resolve(&vt.ty), Ty::Never) && !fits {
                        self.diags.push(err(&vt.span, format!("cannot assign {} to '{}' which is declared as {}", ty_name(&vt.ty), name, ty_name(&aty))));
                        return None;
                    }
                    if ctx.in_fn {
                        let id = ctx.declare(name, aty.clone());
                        out.push(CS::Decl { local: id, name: name.clone(), ty: aty, value: Some(vt) });
                    } else {
                        let cname = format!("still_g_{}__{}", sanitize(&module), sanitize(name));
                        if self.prog.globals.contains_key(&cname) {
                            self.diags.push(err(nsp, format!("global '{}' is already defined", name)));
                            return None;
                        }
                        self.prog.globals.insert(cname.clone(), GlobalVar { cname: cname.clone(), module: module.clone(), name: name.clone(), ty: aty.clone() });
                        self.prog.global_order.push(cname.clone());
                        self.gmap.insert((module.clone(), name.clone()), cname.clone());
                        out.push(CS::Assign { place: Place::Global(cname), value: vt, ty: aty, span: span.clone() });
                    }
                    return Some(());
                }
                let ty = if matches!(resolve(&vt.ty), Ty::NoneTy) {
                    Ty::Var(Rc::new(std::cell::RefCell::new(Some(Ty::NoneTy))))
                } else {
                    vt.ty.clone()
                };
                if ctx.in_fn || ctx.pat_binders.contains(name.as_str()) {
                    // Pattern binders are fresh locals in the arm's own scope
                    // even at top level, so two arms binding the same name
                    // don't collide through a module global (top-level plain
                    // assignments still become globals).
                    let id = ctx.declare(name, ty.clone());
                    out.push(CS::Decl { local: id, name: name.clone(), ty, value: Some(vt) });
                } else {
                    let cname = format!("still_g_{}__{}", sanitize(&module), sanitize(name));
                    if self.prog.globals.contains_key(&cname) {
                        self.diags.push(err(nsp, format!("global '{}' is already defined", name)));
                        return None;
                    }
                    self.prog.globals.insert(cname.clone(), GlobalVar { cname: cname.clone(), module: module.clone(), name: name.clone(), ty: ty.clone() });
                    self.prog.global_order.push(cname.clone());
                    self.gmap.insert((module.clone(), name.clone()), cname.clone());
                    out.push(CS::Assign { place: Place::Global(cname), value: vt, ty, span: span.clone() });
                }
                return Some(());
            }
            let (place, pty) = self.check_target(ctx, &targets[0])?;
            let vt = self.infer(ctx, value)?;
            let bound = self.compat(&vt.ty, &pty);
            if !bound && assign_ok(&vt.ty, &pty).is_none() && !coerce_holes(&vt.ty, &pty) {
                self.diags.push(err(&vt.span, format!("cannot assign {} here (this holds {})", ty_name(&vt.ty), ty_name(&pty))));
                return None;
            }
            let pty = if matches!(resolve(&pty), Ty::Var(_)) { resolve(&vt.ty) } else { pty };
            out.push(CS::Assign { place, value: vt, ty: pty, span: span.clone() });
            return Some(());
        }
        let vt = self.infer(ctx, value)?;
        let ts = match resolve(&vt.ty) {
            Ty::Tuple(ts) if ts.len() == targets.len() => (*ts).clone(),
            Ty::Tuple(ts) => {
                self.diags.push(err(span, format!("got {} values but {} names", ts.len(), targets.len())));
                return None;
            }
            other => {
                self.diags.push(err(span, format!("cannot unpack {} into {} names (the right side must be a tuple)", ty_name(&other), targets.len())));
                return None;
            }
        };
        let module = ctx.module.clone();
        let fresh = |c: &Checker, t: &Expr| match t {
            Expr::Ident(n, _) => c.lookup_local(ctx, n).is_none() && !c.gmap.contains_key(&(module.clone(), n.clone())),
            _ => false,
        };
        let direct = matches!(&vt.x, X::TupleLit(_)) && targets.iter().all(|t| fresh(self, t));
        if direct {
            if let X::TupleLit(items) = &vt.x {
                let items = items.clone();
                for (i, t) in targets.iter().enumerate() {
                    if let Expr::Ident(name, _) = t {
                        let item = items[i].clone();
                        let ty = if matches!(resolve(&item.ty), Ty::NoneTy) {
                            Ty::Var(Rc::new(std::cell::RefCell::new(Some(Ty::NoneTy))))
                        } else {
                            item.ty.clone()
                        };
                        if ctx.in_fn {
                            let id = ctx.declare(name, ty.clone());
                            out.push(CS::Decl { local: id, name: name.clone(), ty, value: Some(item) });
                        } else {
                            let cname = format!("still_g_{}__{}", sanitize(&module), sanitize(name));
                            if self.prog.globals.contains_key(&cname) {
                                continue;
                            }
                            self.prog.globals.insert(cname.clone(), GlobalVar { cname: cname.clone(), module: module.clone(), name: name.clone(), ty: ty.clone() });
                            self.prog.global_order.push(cname.clone());
                            self.gmap.insert((module.clone(), name.clone()), cname.clone());
                            out.push(CS::Assign { place: Place::Global(cname), value: item, ty, span: span.clone() });
                        }
                    }
                }
            }
            return Some(());
        }
        let temp_name = format!("$t{}", ctx.locals.len());
        let tuple_ty = vt.ty.clone();
        let id = ctx.declare(&temp_name, tuple_ty.clone());
        out.push(CS::Decl { local: id, name: temp_name, ty: tuple_ty.clone(), value: Some(vt) });
        for (i, t) in targets.iter().enumerate() {
            let item_tx = Tx {
                ty: ts[i].clone(),
                span: t.span(),
                x: X::Index(
                    Box::new(Tx { ty: tuple_ty.clone(), span: t.span(), x: X::Local(id) }),
                    Box::new(Tx { ty: Ty::Int, span: t.span(), x: X::Int(i as i64) }),
                ),
            };
            if let Expr::Ident(name, nsp) = t {
                if ctx.lookup(name).is_none() && !self.gmap.contains_key(&(module.clone(), name.clone())) {
                    let ty = if matches!(resolve(&item_tx.ty), Ty::NoneTy) {
                        Ty::Var(Rc::new(std::cell::RefCell::new(Some(Ty::NoneTy))))
                    } else {
                        item_tx.ty.clone()
                    };
                    if ctx.in_fn {
                        let id2 = ctx.declare(name, ty.clone());
                        out.push(CS::Decl { local: id2, name: name.clone(), ty, value: Some(item_tx) });
                    } else {
                        let cname = format!("still_g_{}__{}", sanitize(&module), sanitize(name));
                        if !self.prog.globals.contains_key(&cname) {
                            self.prog.globals.insert(cname.clone(), GlobalVar { cname: cname.clone(), module: module.clone(), name: name.clone(), ty: ty.clone() });
                            self.prog.global_order.push(cname.clone());
                            self.gmap.insert((module.clone(), name.clone()), cname.clone());
                            out.push(CS::Assign { place: Place::Global(cname), value: item_tx, ty, span: span.clone() });
                        }
                    }
                    continue;
                }
                let _ = nsp;
            }
            let (place, pty) = self.check_target(ctx, t)?;
            if assign_ok(&item_tx.ty, &pty).is_none() && !coerce_holes(&item_tx.ty, &pty) {
                self.diags.push(err(&item_tx.span, format!("cannot assign {} to '{}' which holds {}", ty_name(&item_tx.ty), expr_name(t), ty_name(&pty))));
                return None;
            }
            out.push(CS::Assign { place, value: item_tx, ty: pty, span: span.clone() });
        }
        Some(())
    }

    fn lookup_local(&self, ctx: &Ctx, name: &str) -> Option<u32> {
        ctx.lookup(name)
    }

    fn check_target(&mut self, ctx: &mut Ctx, t: &Expr) -> Option<(Place, Ty)> {
        match t {
            Expr::Ident(name, nsp) => {
                if let Some(id) = ctx.lookup(name) {
                    if ctx.loop_vars.contains(name) {
                        self.diags.push(err(nsp, format!("cannot assign to the loop variable '{}'", name)));
                        return None;
                    }
                    return Some((Place::Local(id), ctx.locals[id as usize].1.clone()));
                }
                let module = ctx.module.clone();
                if let Some(cname) = self.gmap.get(&(module.clone(), name.to_string())).cloned() {
                    if ctx.in_fn && !ctx.global_names.contains(name) {
                        self.diags.push(err(nsp, format!("cannot assign to the global '{}' from inside a function (write 'global {}' first)", name, name)));
                        return None;
                    }
                    return Some((Place::Global(cname.clone()), self.prog.globals[&cname].ty.clone()));
                }
                self.diags.push(err(nsp, format!("unknown name '{}' (define it before assigning to it)", name)));
                None
            }
            Expr::Index(obj, idx, sp) => {
                if let Expr::Member(o, fname, _) = obj.as_ref() {
                    let ot = self.infer(ctx, o)?;
                    if let Ty::Named(qual) = resolve(&ot.ty) {
                        if let Some(si) = self.prog.structs.get(&qual).cloned() {
                            if si.is_soa {
                                let it = self.want_int(ctx, idx, "soa index")?;
                                let fty = match si.fields.iter().find(|(n, _)| n == fname) {
                                    Some((_, t)) => t.clone(),
                                    None => {
                                        self.diags.push(err(sp, format!("type '{}' has no field '{}'", si.name, fname)));
                                        return None;
                                    }
                                };
                                let base = self.place_of(ctx, o)?;
                                return Some((Place::SoaElem(Box::new(base), fname.clone(), Box::new(it)), fty));
                            }
                        }
                    }
                }
                let ot = self.infer(ctx, obj)?;
                let it = self.infer(ctx, idx)?;
                match resolve(&ot.ty) {
                    Ty::List(elem) => {
                        if !matches!(resolve(&it.ty), Ty::Int | Ty::Never) {
                            self.diags.push(err(sp, format!("list indexes must be int, not {}", ty_name(&it.ty))));
                            return None;
                        }
                        Some((Place::Index(Box::new(self.place_of(ctx, obj)?), Box::new(it)), (*elem).clone()))
                    }
                    Ty::Dict(k, v) => {
                        if !self.compat(&it.ty, &k) {
                            self.diags.push(err(sp, format!("this dict is keyed by {}, not {}", ty_name(&k), ty_name(&it.ty))));
                            return None;
                        }
                        Some((Place::Index(Box::new(self.place_of(ctx, obj)?), Box::new(it)), (*v).clone()))
                    }
                    Ty::Str => {
                        self.diags.push(err(sp, "strings cannot be changed after they are made".into()));
                        None
                    }
                    Ty::Tuple(_) => {
                        self.diags.push(err(sp, "tuples cannot be changed after they are made".into()));
                        None
                    }
                    other => {
                        self.diags.push(err(sp, format!("cannot assign into {}", ty_name(&other))));
                        None
                    }
                }
            }
            Expr::Member(obj, name, sp) => {
                let ot = self.infer(ctx, obj)?;
                match resolve(&ot.ty) {
                    Ty::Named(qual) => {
                        if self.prog.enums.contains_key(&qual) {
                            self.diags.push(err(sp, "enum values are immutable; construct a new value with a variant constructor instead of writing fields".into()));
                            return None;
                        }
                        let si = self.prog.structs.get(&qual).cloned();
                        if let Some(si) = si {
                            if si.is_soa {
                                self.diags.push(err(sp, format!("soa field '{}' needs an index: parts.{}[i] = value", name, name)));
                                return None;
                            }
                            for (n, fty) in &si.fields {
                                if n == name {
                                    return Some((Place::Field(Box::new(self.place_of(ctx, obj)?), name.clone()), fty.clone()));
                                }
                            }
                            let fields: Vec<String> = si.fields.iter().map(|(n, _)| n.clone()).collect();
                            let hint = did_you_mean(name, &fields).map(|h| format!(" (did you mean '{}'?)", h)).unwrap_or_default();
                            self.diags.push(err(sp, format!("type '{}' has no field '{}'{}", si.name, name, hint)));
                            return None;
                        }
                        self.diags.push(err(sp, format!("unknown type '{}'", qual)));
                        None
                    }
                    other => {
                        self.diags.push(err(sp, format!("cannot set fields on {}", ty_name(&other))));
                        None
                    }
                }
            }
            other => {
                self.diags.push(err(&other.span(), "cannot assign to this".into()));
                None
            }
        }
    }

    fn place_of(&mut self, ctx: &mut Ctx, e: &Expr) -> Option<Place> {
        match e {
            Expr::Ident(name, _) => {
                if let Some(id) = ctx.lookup(name) {
                    return Some(Place::Local(id));
                }
                let module = ctx.module.clone();
                if let Some(cname) = self.gmap.get(&(module.clone(), name.to_string())).cloned() {
                    return Some(Place::Global(cname));
                }
                None
            }
            Expr::Index(obj, idx, _) => {
                if let Expr::Member(o, fname, _) = obj.as_ref() {
                    let ot = self.infer(ctx, o)?;
                    if let Ty::Named(qual) = resolve(&ot.ty) {
                        if self.prog.structs.get(&qual).map(|s| s.is_soa).unwrap_or(false) {
                            let it = self.want_int(ctx, idx, "soa index")?;
                            let base = self.place_of(ctx, o)?;
                            return Some(Place::SoaElem(Box::new(base), fname.clone(), Box::new(it)));
                        }
                    }
                }
                let ot = self.infer(ctx, obj)?;
                let it = self.infer(ctx, idx)?;
                match resolve(&ot.ty) {
                    Ty::List(_) | Ty::Dict(..) => Some(Place::Index(Box::new(self.place_of(ctx, obj)?), Box::new(it))),
                    _ => None,
                }
            }
            Expr::Member(obj, name, _) => {
                let ot = self.infer(ctx, obj)?;
                match resolve(&ot.ty) {
                    Ty::Named(qual) if self.prog.structs.get(&qual).map(|s| !s.is_soa).unwrap_or(false) => {
                        Some(Place::Field(Box::new(self.place_of(ctx, obj)?), name.clone()))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn expr_name(e: &Expr) -> String {
    match e {
        Expr::Ident(n, _) => n.clone(),
        Expr::Index(..) => "this index".into(),
        Expr::Member(..) => "this field".into(),
        _ => "this".into(),
    }
}

impl Checker {
    fn convert_externs(&mut self) {
        let mut names: Vec<String> = self.externs.keys().cloned().collect();
        names.sort();
        for n in names {
            let e = self.externs[&n].clone();
            let mut c_params = Vec::new();
            for (_, a) in &e.params {
                c_params.push(match a {
                    TypeAnnot::Int => Ty::Int,
                    TypeAnnot::Float => Ty::Float,
                    TypeAnnot::Bool => Ty::Bool,
                    _ => Ty::Str,
                });
            }
            let c_ret = match &e.ret {
                Some(TypeAnnot::Int) => Ty::Int,
                Some(TypeAnnot::Float) => Ty::Float,
                Some(TypeAnnot::Bool) => Ty::Bool,
                Some(TypeAnnot::Str) => Ty::Str,
                _ => Ty::NoneTy,
            };
            self.prog.externs.push(ExternDecl {
                name: e.name.clone(),
                c_params,
                variadic: e.variadic,
                c_ret,
            });
        }
    }

    fn finalize_types(&mut self) {
        let mut problems: Vec<(String, Span)> = Vec::new();
        let globals = self.prog.global_order.clone();
        for g in &globals {
            let gv = self.prog.globals[g].clone();
            if let Ty::Var(c) = &resolve(&gv.ty) {
                if c.borrow().is_none() {
                    problems.push((format!("cannot infer the type of '{}' (annotate it, e.g. {}: int = 0)", gv.name, gv.name), gv_placeholder_span()));
                    continue;
                }
                if contains_unresolved(&resolve_deep(&gv.ty)) {
                    problems.push((format!("cannot fully infer the type of '{}'", gv.name), gv_placeholder_span()));
                }
            }
        }
        let fns: Vec<usize> = (0..self.prog.funcs.len()).collect();
        for i in fns {
            let f = self.prog.funcs[i].clone();
            for (pn, pt) in &f.params {
                if contains_unresolved(&resolve_deep(pt)) {
                    problems.push((
                        format!("cannot infer the type of parameter '{}' in '{}' (annotate it, e.g. {}: int)", pn, f.source_name, pn),
                        f_placeholder_span(),
                    ));
                }
            }
            if contains_unresolved(&resolve_deep(&f.ret)) {
                problems.push((format!("cannot infer the return type of '{}'", f.source_name), f_placeholder_span()));
            }
            collect_cs_unresolved(&f.body, &mut problems, &f.source_name);
        }
        let inits = self.prog.module_inits.clone();
        for (m, cs) in &inits {
            collect_cs_unresolved(cs, &mut problems, m);
        }
        collect_cs_unresolved(&self.prog.main_body.clone(), &mut problems, "main");
        for (msg, sp) in problems {
            self.diags.push(err(&sp, msg));
        }
    }
}

fn gv_placeholder_span() -> Span {
    Span::new(std::rc::Rc::from("types"), 1, 1)
}

fn f_placeholder_span() -> Span {
    Span::new(std::rc::Rc::from("types"), 1, 1)
}

fn contains_unresolved(t: &Ty) -> bool {
    match resolve(t) {
        Ty::Var(c) => c.borrow().is_none(),
        Ty::Nullable(b) => contains_unresolved(&b),
        Ty::List(e) => contains_unresolved(&e),
        Ty::Dict(k, v) => contains_unresolved(&k) || contains_unresolved(&v),
        Ty::Tuple(ts) => ts.iter().any(contains_unresolved),
        _ => false,
    }
}

fn collect_cs_unresolved(stmts: &[CS], out: &mut Vec<(String, Span)>, ctx: &str) {
    for s in stmts {
        match s {
            CS::Decl { name, ty, .. } => {
                if contains_unresolved(&resolve_deep(ty)) {
                    out.push((
                        format!("cannot infer the type of '{}' in {} (annotate it, e.g. {}: [int] = [] — or the list stays a mystery)", name, ctx, name),
                        gv_placeholder_span(),
                    ));
                }
            }
            CS::Assign { ty, .. } | CS::AugAssign { ty, .. } => {
                if contains_unresolved(&resolve_deep(ty)) {
                    out.push(("cannot infer a type in this assignment (empty lists need a hint: x: [int] = [])".into(), gv_placeholder_span()));
                }
            }
            CS::If { then, els, .. } => {
                collect_cs_unresolved(then, out, ctx);
                collect_cs_unresolved(els, out, ctx);
            }
            CS::While { body, .. } => collect_cs_unresolved(body, out, ctx),
            CS::ForList { body, .. }
            | CS::ForRange { body, .. }
            | CS::ForEnum { body, .. }
            | CS::ForStr { body, .. }
            | CS::ForSoa { body, .. }
            | CS::ForPairs { body, .. } => collect_cs_unresolved(body, out, ctx),
            _ => {}
        }
    }
}
