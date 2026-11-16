#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Function, Let, Return, If, Otherwise, While, When, Show, As,
    Module, Structure, Enumeration, Implementation, Protocol,
    Unsafe, Assembly, Static, Mutable, External,
    No, Standard, Entry, Panic, Takes, Gives, SelfKw,
    Rule, Emit, True, False,
    Output, Input, Interrupt, Halt, Operation,
    Then, For, In, Each,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeTok {
    Unsigned8, Unsigned16, Unsigned32, Unsigned64,
    Integer8, Integer16, Integer32, Integer64,
    Float32, Float64, Boolean, Size, String,
}

impl TypeTok {
    pub fn from_canonical(s: &str) -> Option<TypeTok> {
        use TypeTok::*;
        Some(match s {
            "unsigned8"  => Unsigned8,
            "unsigned16" => Unsigned16,
            "unsigned32" => Unsigned32,
            "unsigned64" => Unsigned64,
            "integer8"   => Integer8,
            "integer16"  => Integer16,
            "integer32"  => Integer32,
            "integer64"  => Integer64,
            "float32"    => Float32,
            "float64"    => Float64,
            "boolean"    => Boolean,
            "size"       => Size,
            "string"     => String,
            _ => return None,
        })
    }
}

impl Keyword {
    pub fn from_canonical(s: &str) -> Option<Keyword> {
        use Keyword::*;
        Some(match s {
            "function"       => Function,
            "let"            => Let,
            "return"         => Return,
            "if"             => If,
            "otherwise"      => Otherwise,
            "while"          => While,
            "when"           => When,
            "show"           => Show,
            "as"             => As,
            "module"         => Module,
            "structure"      => Structure,
            "enumeration"    => Enumeration,
            "implementation" => Implementation,
            "protocol"       => Protocol,
            "unsafe"         => Unsafe,
            "assembly"       => Assembly,
            "static"         => Static,
            "mutable"        => Mutable,
            "external"       => External,
            "no"             => No,
            "standard"       => Standard,
            "entry"          => Entry,
            "panic"          => Panic,
            "takes"          => Takes,
            "gives"          => Gives,
            "self"           => SelfKw,
            "rule"           => Rule,
            "emit"           => Emit,
            "true"           => True,
            "false"          => False,
            "output"         => Output,
            "input"          => Input,
            "interrupt"      => Interrupt,
            "halt"           => Halt,
            "operation"      => Operation,
            "then"           => Then,
            "for"            => For,
            "in"             => In,
            "each"           => Each,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    IntLit(u64),
    FloatLit(f64),
    StrLit(String),
    BoolLit(bool),
    Kw(Keyword),
    Ty(TypeTok),

    Newline, Indent, Dedent,

    Eq, EqEq, Ne, Lt, Gt, Le, Ge,
    Plus, Minus, Star, Slash, Percent,
    Amp, Pipe, Caret, Shl, Shr, Tilde,
    Colon, Comma, Dot, LParen, RParen,
    LBracket, RBracket,
    LBrace,   RBrace,
    Arrow,
    Eof,
}

impl Token {
    pub fn describe(&self) -> String {
        format!("{:?}", self)
    }
}