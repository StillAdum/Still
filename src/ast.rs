use crate::span::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    Named(String),
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
    pub fields: Vec<Field>,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

impl TypeDef {
    pub fn is_soa(&self) -> bool {
        self.attrs.iter().any(|a| a == "soa")
    }
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
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>, Span),
    Member(Box<Expr>, String, Span),
}

#[derive(Clone, Debug)]
pub enum InterpPart {
    Lit(String),
    Code(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    FuncDef(FuncDef),
    TypeDef(TypeDef),
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
            | Expr::Slice(_, _, _, s)
            | Expr::Member(_, _, s) => s.clone(),
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
    Never,
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
        other => other,
    }
}

pub fn ty_eq(a: &Ty, b: &Ty) -> bool {
    match (resolve(a), resolve(b)) {
        (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float) | (Ty::Bool, Ty::Bool) | (Ty::Str, Ty::Str) | (Ty::NoneTy, Ty::NoneTy) | (Ty::Never, Ty::Never) => true,
        (Ty::Named(a), Ty::Named(b)) => a == b,
        (Ty::Nullable(a), Ty::Nullable(b)) => ty_eq(&a, &b),
        (Ty::List(a), Ty::List(b)) => ty_eq(&a, &b),
        (Ty::Dict(ak, av), Ty::Dict(bk, bv)) => ty_eq(&ak, &bk) && ty_eq(&av, &bv),
        (Ty::Tuple(a), Ty::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| ty_eq(x, y)),
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
        Ty::Never => "never".into(),
    }
}

pub fn is_ref_ty(t: &Ty) -> bool {
    matches!(resolve(t), Ty::Str | Ty::List(_) | Ty::Dict(..) | Ty::Named(_))
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
    SoaNew(String, Box<Tx>),
    Neg(Box<Tx>),
    Not(Box<Tx>),
    Arith(BinOp, Box<Tx>, Box<Tx>),
    And(Box<Tx>, Box<Tx>),
    Or(Box<Tx>, Box<Tx>),
    Cmp(CmpOp, Box<Tx>, Box<Tx>),
    In(Box<Tx>, Box<Tx>, bool),
    IsNone(Box<Tx>, bool),
    Cond(Box<Tx>, Box<Tx>, Box<Tx>),
    CallFn(usize, Vec<Tx>),
    CallExtern(String, Vec<Tx>),
    Builtin(Bi, Vec<Tx>),
    Method(Box<Tx>, String, Vec<Tx>),
    Index(Box<Tx>, Box<Tx>),
    Slice(Box<Tx>, Option<Box<Tx>>, Option<Box<Tx>>),
    Field(Box<Tx>, String),
    Unwrap(Box<Tx>),
    AsStr(Box<Tx>),
    AsFloat(Box<Tx>),
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
    pub exported: bool,
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
        Ty::Named(n) => n.replace("::", "__"),
        Ty::Never => "x".into(),
    }
}

pub fn annot_to_ty(a: &TypeAnnot, structs: &HashMap<String, StructInfo>) -> Result<Ty, String> {
    Ok(match a {
        TypeAnnot::Int => Ty::Int,
        TypeAnnot::Float => Ty::Float,
        TypeAnnot::Bool => Ty::Bool,
        TypeAnnot::Str => Ty::Str,
        TypeAnnot::None => Ty::NoneTy,
        TypeAnnot::List(e) => Ty::List(Rc::new(annot_to_ty(e, structs)?)),
        TypeAnnot::Dict(k, v) => Ty::Dict(Rc::new(annot_to_ty(k, structs)?), Rc::new(annot_to_ty(v, structs)?)),
        TypeAnnot::Tuple(ts) => Ty::Tuple(Rc::new(ts.iter().map(|t| annot_to_ty(t, structs)).collect::<Result<_, _>>()?)),
        TypeAnnot::Nullable(b) => Ty::Nullable(Box::new(annot_to_ty(b, structs)?)),
        TypeAnnot::Named(n) => {
            if structs.contains_key(n) {
                Ty::Named(n.clone())
            } else if let Some((qual, _)) = structs.iter().find(|(k, _)| k.ends_with(&format!("::{}", n)) || k.as_str() == n) {
                Ty::Named(qual.clone())
            } else {
                return Err(format!("unknown type '{}'", n));
            }
        }
    })
}
