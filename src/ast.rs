use crate::span::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Map arbitrary characters to valid C identifier characters. Used for
/// struct/function C names and for `ty_tag` of named types (generic
/// instantiations contain '<' '>' ','). Injective: every output sequence
/// decodes back to exactly one input string.
pub fn sanitize(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push_str(&format!("_u{:04x}_", ch as u32));
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeAnnot {
    Int,
    Float,
    Bool,
    Str,
    None,
    List(Box<TypeAnnot>),
    Dict(Box<TypeAnnot>, Box<TypeAnnot>),
    Tuple(Vec<TypeAnnot>),
    Nullable(Box<TypeAnnot>),
    Any,
    Named(String),
    /// `SlotMap<int>` / `Result<int, str>` — a generic type with explicit
    /// type arguments.
    Generic(String, Vec<TypeAnnot>),
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub annot: Option<TypeAnnot>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FuncDef {
    pub attrs: Vec<String>,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeAnnot>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub annot: TypeAnnot,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub attrs: Vec<String>,
    pub name: String,
    /// Type parameters: `type SlotMap<T>` -> ["T"]. Empty for plain types.
    pub params: Vec<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

impl TypeDef {
    pub fn is_soa(&self) -> bool {
        self.attrs.iter().any(|a| a == "soa")
    }
}

/// One enum variant: `Circle(f32, f32, f32)` — name plus payload types.
#[derive(Clone, Debug)]
pub struct VariantDef {
    pub name: String,
    pub payloads: Vec<TypeAnnot>,
    pub span: Span,
}

/// `enum Shape: Circle(f32, f32, f32) / Rect(f32, f32, f32, f32) / Point`
#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<VariantDef>,
    pub span: Span,
}

/// `trait Drawable:` — method signatures only. Stage B: traits are a
/// compile-time contract; impl methods become real struct methods
/// (static dispatch). Dynamic dispatch is a follow-up.
#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

/// `impl Drawable for Circle:` — one type's implementation of a trait.
#[derive(Clone, Debug)]
pub struct ImplDef {
    pub name: String,
    pub for_type: String,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

/// A registered trait: method signatures with resolved types. `self` is
/// implicit (unannotated) in trait signatures.
#[derive(Clone, Debug)]
pub struct TraitInfo {
    pub module: String,
    pub name: String,
    /// (method name, takes self, param types excluding self, return type)
    pub methods: Vec<(String, bool, Vec<Ty>, Option<Ty>)>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<(String, TypeAnnot)>,
    pub variadic: bool,
    pub ret: Option<TypeAnnot>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Shl,
    Shr,
    Band,
    Bor,
    Bxor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug)]
pub enum Expr {
    IntLit(i64, Span),
    FloatLit(f64, Span),
    BoolLit(bool, Span),
    NoneLit(Span),
    StrLit(String, Span),
    Interp(Vec<InterpPart>, Span),
    Ident(String, Span),
    Tuple(Vec<Expr>, Span),
    ListLit(Vec<Expr>, Span),
    DictLit(Vec<(Expr, Expr)>, Span),
    Unary(UnOp, Box<Expr>, Span),
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    Cmp(Box<Expr>, CmpOp, Box<Expr>, Span),
    And(Box<Expr>, Box<Expr>, Span),
    Or(Box<Expr>, Box<Expr>, Span),
    Not(Box<Expr>, Span),
    In(Box<Expr>, Box<Expr>, bool, Span),
    IsNone(Box<Expr>, bool, Span),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>, Option<Box<Expr>>, Span),
    Member(Box<Expr>, String, Span),
    /// `x?` — runtime-checked unwrap of a nullable value (panics on none).
    UnwrapQ(Box<Expr>, Span),
}

#[derive(Clone, Debug)]
pub enum InterpPart {
    Lit(String),
    Code(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    BNot,
}

#[derive(Clone, Debug)]
pub struct MatchCase {
    pub values: Vec<Expr>,
    pub wildcard: bool,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    FuncDef(FuncDef),
    TypeDef(TypeDef),
    EnumDef(EnumDef),
    TraitDef(TraitDef),
    ImplDef(ImplDef),
    Extern(Vec<ExternFn>),
    Use {
        target: String,
        alias: Option<String>,
        names: Option<Vec<String>>,
        span: Span,
    },
    Assign {
        targets: Vec<Expr>,
        op: Option<BinOp>,
        value: Expr,
        annot: Option<TypeAnnot>,
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        span: Span,
    },
    If {
        arms: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        vars: Vec<String>,
        iter: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Global(Vec<String>, Span),
    Match {
        subject: Expr,
        arms: Vec<MatchCase>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Pass(Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s)
            | Expr::FloatLit(_, s)
            | Expr::BoolLit(_, s)
            | Expr::NoneLit(s)
            | Expr::StrLit(_, s)
            | Expr::Interp(_, s)
            | Expr::Ident(_, s)
            | Expr::Tuple(_, s)
            | Expr::ListLit(_, s)
            | Expr::DictLit(_, s)
            | Expr::Unary(_, _, s)
            | Expr::Binary(_, _, _, s)
            | Expr::Cmp(_, _, _, s)
            | Expr::And(_, _, s)
            | Expr::Or(_, _, s)
            | Expr::Not(_, s)
            | Expr::In(_, _, _, s)
            | Expr::IsNone(_, _, s)
            | Expr::Cond(_, _, _, s)
            | Expr::Call(_, _, s)
            | Expr::Index(_, _, s)
            | Expr::Slice(_, _, _, _, s)
            | Expr::Member(_, _, s)
            | Expr::UnwrapQ(_, s) => s.clone(),
        }
    }
}

pub type VarCell = Rc<RefCell<Option<Ty>>>;

#[derive(Clone, Debug)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    NoneTy,
    Var(VarCell),
    Nullable(Box<Ty>),
    List(Rc<Ty>),
    Dict(Rc<Ty>, Rc<Ty>),
    Tuple(Rc<Vec<Ty>>),
    Named(String),
    Any,
    Never,
    /// Function type: (param types, return type). Used for first-class
    /// function values with static dispatch (no capture yet).
    Func(Vec<Ty>, Box<Ty>),
}

pub fn resolve(t: &Ty) -> Ty {
    match t {
        Ty::Var(cell) => match &*cell.borrow() {
            Some(inner) => resolve(inner),
            None => t.clone(),
        },
        _ => t.clone(),
    }
}

pub fn resolve_deep(t: &Ty) -> Ty {
    match resolve(t) {
        Ty::Nullable(b) => Ty::Nullable(Box::new(resolve_deep(&b))),
        Ty::List(e) => Ty::List(Rc::new(resolve_deep(&e))),
        Ty::Dict(k, v) => Ty::Dict(Rc::new(resolve_deep(&k)), Rc::new(resolve_deep(&v))),
        Ty::Tuple(ts) => Ty::Tuple(Rc::new(ts.iter().map(resolve_deep).collect())),
Ty::Func(ps, r) => Ty::Func(ps.iter().map(resolve_deep).collect(), Box::new(resolve_deep(&r))),
        other => other,
    }
}

pub fn ty_eq(a: &Ty, b: &Ty) -> bool {
    match (resolve(a), resolve(b)) {
        (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float) | (Ty::Bool, Ty::Bool) | (Ty::Str, Ty::Str) | (Ty::NoneTy, Ty::NoneTy) | (Ty::Never, Ty::Never) | (Ty::Any, Ty::Any) => true,
        (Ty::Named(a), Ty::Named(b)) => a == b,
        (Ty::Nullable(a), Ty::Nullable(b)) => ty_eq(&a, &b),
        (Ty::List(a), Ty::List(b)) => ty_eq(&a, &b),
        (Ty::Dict(ak, av), Ty::Dict(bk, bv)) => ty_eq(&ak, &bk) && ty_eq(&av, &bv),
        (Ty::Tuple(a), Ty::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| ty_eq(x, y)),
(Ty::Func(a_params, a_ret), Ty::Func(b_params, b_ret)) => a_params.len() == b_params.len() && a_params.iter().zip(b_params.iter()).all(|(x, y)| ty_eq(x, y)) && ty_eq(&a_ret, &b_ret),
        _ => false,
    }
}

pub fn ty_name(t: &Ty) -> String {
    match resolve(t) {
        Ty::Int => "int".into(),
        Ty::Float => "float".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "str".into(),
        Ty::NoneTy => "none".into(),
        Ty::Var(_) => "unknown".into(),
        Ty::Nullable(b) => format!("{}?", ty_name(&b)),
        Ty::List(e) => format!("[{}]", ty_name(&e)),
        Ty::Dict(k, v) => format!("{{{}: {}}}", ty_name(&k), ty_name(&v)),
        Ty::Tuple(ts) => {
            let parts: Vec<String> = ts.iter().map(ty_name).collect();
            format!("({})", parts.join(", "))
        }
        Ty::Named(n) => n,
        Ty::Any => "any".into(),
        Ty::Never => "never".into(),
        Ty::Func(params, ret) => {
            let ps: Vec<String> = params.iter().map(ty_name).collect();
            format!("func({}) -> {}", ps.join(", "), ty_name(&ret))
        },
    }
}

pub fn is_ref_ty(t: &Ty) -> bool {
    matches!(resolve(t), Ty::Str | Ty::List(_) | Ty::Dict(..) | Ty::Named(_) | Ty::Any | Ty::Func(..))
}

#[derive(Clone, Debug)]
pub struct Tx {
    pub ty: Ty,
    pub span: Span,
    pub x: X,
}

#[derive(Clone, Debug)]
pub enum X {
    Int(i64),
    Float(f64),
    Bool(bool),
    NoneV,
    Str(String),
    Local(u32),
    Global(String),
    ListLit(Vec<Tx>),
    DictLit(Vec<(Tx, Tx)>),
    TupleLit(Vec<Tx>),
    StructLit(String, Vec<(String, Tx)>),
    /// Enum constructor: `Shape.Circle(1.0, 2.0)` — variant index is the
    /// runtime tag; payloads are stored flat (`Variant_i` fields).
    EnumLit(String, usize, Vec<Tx>),
    SoaNew(String, Box<Tx>),
    Neg(Box<Tx>),
    BNot(Box<Tx>),
    Not(Box<Tx>),
    Arith(BinOp, Box<Tx>, Box<Tx>),
    And(Box<Tx>, Box<Tx>),
    Or(Box<Tx>, Box<Tx>),
    Cmp(CmpOp, Box<Tx>, Box<Tx>),
    In(Box<Tx>, Box<Tx>, bool),
    IsNone(Box<Tx>, bool),
    Cond(Box<Tx>, Box<Tx>, Box<Tx>),
    CallFn(usize, Vec<Tx>),
    CallFuncValue(Box<Tx>, Vec<Tx>),
    CallExtern(String, Vec<Tx>),
    Builtin(Bi, Vec<Tx>),
    Method(Box<Tx>, String, Vec<Tx>),
    Index(Box<Tx>, Box<Tx>),
    /// Runtime-checked unwrap of a nullable (panics on none).
    CheckedUnwrap(Box<Tx>),
    Slice(Box<Tx>, Option<Box<Tx>>, Option<Box<Tx>>, Option<Box<Tx>>),
    Field(Box<Tx>, String),
    Unwrap(Box<Tx>),
    AsStr(Box<Tx>),
    AsFloat(Box<Tx>),
    FuncValue(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bi {
    Print,
    Len,
    Push,
    Pop,
    Keys,
    Pairs,
    StrOf,
    IntOf,
    FloatOf,
    BoolOf,
    Abs,
    Min,
    Max,
    Sum,
    Range,
    Range2,
    Range3,
    Enumerate,
    Reversed,
    TypeOf,
    Input,
    InputPrompt,
    Clock,
    Rand,
    RandBound,
    Seed,
    Check,
    CheckMsg,
    Fail,
    Exit,
    Ord,
    Chr,
    ReadFile,
    WriteFile,
    GetEnv,
    SleepMs,
    Args,
    DictKeyAt,
    ReadDir,
    FileExists,
}

#[derive(Clone, Debug)]
pub enum Place {
    Local(u32),
    Global(String),
    Index(Box<Place>, Box<Tx>),
    Field(Box<Place>, String),
    SoaElem(Box<Place>, String, Box<Tx>),
    TupleIndex(Box<Place>, usize),
}

#[derive(Clone, Debug)]
pub enum CS {
    Decl {
        local: u32,
        name: String,
        ty: Ty,
        value: Option<Tx>,
    },
    Assign {
        place: Place,
        value: Tx,
        ty: Ty,
        span: Span,
    },
    AugAssign {
        place: Place,
        op: BinOp,
        value: Tx,
        ty: Ty,
        span: Span,
    },
    ExprS(Tx),
    If {
        cond: Tx,
        then: Vec<CS>,
        els: Vec<CS>,
    },
    While {
        cond: Tx,
        body: Vec<CS>,
    },
    ForList {
        list: Tx,
        var: u32,
        body: Vec<CS>,
    },
    ForRange {
        start: Tx,
        end: Tx,
        step: Tx,
        var: u32,
        body: Vec<CS>,
    },
    ForEnum {
        list: Tx,
        ivar: u32,
        evar: u32,
        body: Vec<CS>,
    },
    ForAny {
        list: Tx,
        ivar: Option<u32>,
        var: u32,
        body: Vec<CS>,
    },
    ForStr {
        s: Tx,
        var: u32,
        body: Vec<CS>,
    },
    ForSoa {
        soa: Tx,
        var: u32,
        body: Vec<CS>,
    },
    ForPairs {
        list: Tx,
        avar: u32,
        bvar: u32,
        body: Vec<CS>,
    },
    Return {
        value: Option<Tx>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Pass,
}

#[derive(Clone, Debug)]
pub struct CFunc {
    pub cname: String,
    pub source_name: String,
    pub module: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub body: Vec<CS>,
    pub exported: bool,
    pub self_ty: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ExternDecl {
    pub name: String,
    pub c_params: Vec<Ty>,
    pub variadic: bool,
    pub c_ret: Ty,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub module: String,
    pub name: String,
    pub fields: Vec<(String, Ty)>,
    pub defaults: Vec<Option<Tx>>,
    pub is_soa: bool,
    pub methods: Vec<String>,
    pub statics: Vec<String>,
    pub exported: bool,
    /// For instantiated generic types: the concrete type arguments, aligned
    /// with the TypeDef's `params`. Empty for plain types.
    pub ty_args: Vec<Ty>,
}

/// A registered enum: variants with resolved payload types. The enum value
/// itself is `Ty::Named("module::Name")`, laid out as a flat struct with a
/// `tag` int plus one field per payload slot (`Variant_0`, `Variant_1`, …).
#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub module: String,
    pub name: String,
    /// (variant name, payload types) in declaration order; the index IS the
    /// runtime tag.
    pub variants: Vec<(String, Vec<Ty>)>,
    pub exported: bool,
}

impl EnumInfo {
    pub fn variant_index(&self, name: &str) -> Option<usize> {
        self.variants.iter().position(|(n, _)| n == name)
    }
}

#[derive(Clone, Debug)]
pub struct GlobalVar {
    pub cname: String,
    pub module: String,
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Default)]
pub struct Program {
    pub modules: Vec<String>,
    pub funcs: Vec<CFunc>,
    pub externs: Vec<ExternDecl>,
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub traits: HashMap<String, TraitInfo>,
    pub globals: HashMap<String, GlobalVar>,
    pub global_order: Vec<String>,
    pub module_inits: Vec<(String, Vec<CS>)>,
    pub main_body: Vec<CS>,
    pub debug: bool,
}

pub fn ty_tag(t: &Ty) -> String {
    match resolve(t) {
        Ty::Int => "i".into(),
        Ty::Float => "f".into(),
        Ty::Bool => "b".into(),
        Ty::Str => "s".into(),
        Ty::NoneTy => "n".into(),
        Ty::Any => "a".into(),
        Ty::Var(c) => match &*c.borrow() {
            Some(inner) => ty_tag(&inner.clone()),
            None => "v".into(),
        },
        Ty::Nullable(b) => format!("O{}", ty_tag(&b)),
        Ty::List(e) => format!("L{}", ty_tag(&e)),
        Ty::Dict(k, v) => format!("D{}_{}", ty_tag(&k), ty_tag(&v)),
        Ty::Tuple(ts) => {
            let parts: Vec<String> = ts.iter().map(ty_tag).collect();
            format!("T{}", parts.join("_"))
        }
        Ty::Named(n) => sanitize(&n.replace("::", "__")),
        Ty::Never => "x".into(),
        Ty::Func(params, ret) => {
            let mut tag = String::from("F");
            for p in params {
                tag.push_str(&ty_tag(&p));
            }
            tag.push('_');
            tag.push_str(&ty_tag(&ret));
            tag
        },
    }
}

/// Pack a Still type into the runtime `any` tag: base char in the lowest
/// byte, then child bytes ('l'+child for lists, 'd'+key+value for dicts).
/// Returns None when the type is not boxable (structs, tuples, nullables)
/// or nests deeper than the 8 bytes an int64 tag can hold.
pub fn any_box_tag(t: &Ty) -> Option<i64> {
    fn pack(t: &Ty, out: &mut Vec<u8>) -> bool {
        match resolve(t) {
            Ty::Int | Ty::Never => out.push(b'i'),
            Ty::Float => out.push(b'f'),
            Ty::Bool => out.push(b'b'),
            Ty::Str => out.push(b's'),
            Ty::NoneTy => out.push(b'n'),
            Ty::Any => out.push(b'a'),
            Ty::List(e) => {
                out.push(b'l');
                if !pack(&e, out) {
                    return false;
                }
            }
            Ty::Dict(k, v) => {
                out.push(b'd');
                if !pack(&k, out) {
                    return false;
                }
                if !pack(&v, out) {
                    return false;
                }
            }
            _ => return false,
        }
        out.len() <= 8
    }
    let mut out = Vec::new();
    if !pack(t, &mut out) {
        return None;
    }
    let mut tag: i64 = 0;
    for (i, b) in out.iter().enumerate() {
        tag |= (*b as i64) << (8 * i);
    }
    Some(tag)
}

pub fn annot_to_ty(a: &TypeAnnot, structs: &HashMap<String, StructInfo>) -> Result<Ty, String> {
    Ok(match a {
        TypeAnnot::Int => Ty::Int,
        TypeAnnot::Float => Ty::Float,
        TypeAnnot::Bool => Ty::Bool,
        TypeAnnot::Str => Ty::Str,
        TypeAnnot::None => Ty::NoneTy,
        TypeAnnot::Any => Ty::Any,
        TypeAnnot::List(e) => Ty::List(Rc::new(annot_to_ty(e, structs)?)),
        TypeAnnot::Dict(k, v) => Ty::Dict(Rc::new(annot_to_ty(k, structs)?), Rc::new(annot_to_ty(v, structs)?)),
        TypeAnnot::Tuple(ts) => Ty::Tuple(Rc::new(ts.iter().map(|t| annot_to_ty(t, structs)).collect::<Result<_, _>>()?)),
        TypeAnnot::Nullable(b) => {
            let inner = annot_to_ty(b, structs)?;
            if matches!(inner, Ty::Any) {
                return Err("'any?' is not needed; any can already hold none".into());
            }
            Ty::Nullable(Box::new(inner))
        }
        TypeAnnot::Named(n) => {
            if structs.contains_key(n) {
                Ty::Named(n.clone())
            } else if let Some((qual, _)) = structs.iter().find(|(k, _)| k.ends_with(&format!("::{}", n)) || k.as_str() == n) {
                Ty::Named(qual.clone())
            } else {
                return Err(format!("unknown type '{}'", n));
            }
        }
        // Generic type args must go through the checker's resolver (it owns
        // the TypeDef table and the instantiation cache). This context only
        // sees fully substituted types.
        TypeAnnot::Generic(n, _) => {
            return Err(format!("type '{}' needs type arguments here (resolve generics through the checker)", n));
        }
    })
}

/// Canonical string form of an annotation (user syntax). Used to carry
/// generic type arguments from the parser into the checker through the
/// callee-name string, and for type-arg error messages.
pub fn annot_str(a: &TypeAnnot) -> String {
    match a {
        TypeAnnot::Int => "int".into(),
        TypeAnnot::Float => "float".into(),
        TypeAnnot::Bool => "bool".into(),
        TypeAnnot::Str => "str".into(),
        TypeAnnot::None => "none".into(),
        TypeAnnot::Any => "any".into(),
        TypeAnnot::List(e) => format!("[{}]", annot_str(e)),
        TypeAnnot::Dict(k, v) => format!("{{{}: {}}}", annot_str(k), annot_str(v)),
        TypeAnnot::Tuple(ts) => format!("({})", ts.iter().map(annot_str).collect::<Vec<_>>().join(", ")),
        TypeAnnot::Nullable(b) => format!("{}?", annot_str(b)),
        TypeAnnot::Named(n) => n.clone(),
        TypeAnnot::Generic(n, args) => format!("{}<{}>", n, args.iter().map(annot_str).collect::<Vec<_>>().join(", ")),
    }
}

/// Parse the canonical string form produced by `annot_str` back into a
/// `TypeAnnot`. Returns None on malformed input (callers report the error).
pub fn parse_annot_str(s: &str) -> Option<TypeAnnot> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0usize;
    let r = parse_annot_str_at(&chars, &mut pos)?;
    if pos != chars.len() {
        return None;
    }
    Some(r)
}

fn parse_annot_str_at(chars: &[char], pos: &mut usize) -> Option<TypeAnnot> {
    skip_ws(chars, pos);
    // [T]
    if chars.get(*pos) == Some(&'[') {
        *pos += 1;
        let inner = parse_annot_str_at(chars, pos)?;
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&']') {
            return None;
        }
        *pos += 1;
        return with_nullable(chars, pos, TypeAnnot::List(Box::new(inner)));
    }
    // {K: V}
    if chars.get(*pos) == Some(&'{') {
        *pos += 1;
        let k = parse_annot_str_at(chars, pos)?;
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&':') {
            return None;
        }
        *pos += 1;
        let v = parse_annot_str_at(chars, pos)?;
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&'}') {
            return None;
        }
        *pos += 1;
        return with_nullable(chars, pos, TypeAnnot::Dict(Box::new(k), Box::new(v)));
    }
    // (A, B) tuple
    if chars.get(*pos) == Some(&'(') {
        *pos += 1;
        let mut parts = Vec::new();
        skip_ws(chars, pos);
        if chars.get(*pos) == Some(&')') {
            *pos += 1;
            return with_nullable(chars, pos, TypeAnnot::Tuple(parts));
        }
        loop {
            parts.push(parse_annot_str_at(chars, pos)?);
            skip_ws(chars, pos);
            match chars.get(*pos) {
                Some(',') => {
                    *pos += 1;
                }
                Some(')') => {
                    *pos += 1;
                    break;
                }
                _ => return None,
            }
        }
        if parts.len() == 1 {
            let inner = parts.pop().unwrap();
            return with_nullable(chars, pos, inner);
        }
        return with_nullable(chars, pos, TypeAnnot::Tuple(parts));
    }
    // ident (possibly generic Name<...>)
    let start = *pos;
    while chars.get(*pos).map(|c| c.is_ascii_alphanumeric() || *c == '_').unwrap_or(false) {
        *pos += 1;
    }
    if start == *pos {
        return None;
    }
    let name: String = chars[start..*pos].iter().collect();
    let base = match name.as_str() {
        "int" => TypeAnnot::Int,
        "float" => TypeAnnot::Float,
        "bool" => TypeAnnot::Bool,
        "str" => TypeAnnot::Str,
        "none" => TypeAnnot::None,
        "any" => TypeAnnot::Any,
        _ => {
            skip_ws(chars, pos);
            if chars.get(*pos) == Some(&'<') {
                *pos += 1;
                let mut args = Vec::new();
                loop {
                    args.push(parse_annot_str_at(chars, pos)?);
                    skip_ws(chars, pos);
                    match chars.get(*pos) {
                        Some(',') => {
                            *pos += 1;
                        }
                        Some('>') => {
                            *pos += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
                TypeAnnot::Generic(name, args)
            } else {
                TypeAnnot::Named(name)
            }
        }
    };
    with_nullable(chars, pos, base)
}

fn with_nullable(chars: &[char], pos: &mut usize, base: TypeAnnot) -> Option<TypeAnnot> {
    skip_ws(chars, pos);
    if chars.get(*pos) == Some(&'?') {
        *pos += 1;
        Some(TypeAnnot::Nullable(Box::new(base)))
    } else {
        Some(base)
    }
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while chars.get(*pos).map(|c| c.is_whitespace()).unwrap_or(false) {
        *pos += 1;
    }
}
