use crate::command::MiCommand;
use crate::value::EmptyReply;
use serde::{Deserialize, Serialize, Serializer};

/// Frame under which a varobj expression is evaluated.
pub enum VarFrame {
    Current,
    Address(String),
}
impl Serialize for VarFrame {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            VarFrame::Current => s.serialize_str("*"),
            VarFrame::Address(a) => s.serialize_str(a),
        }
    }
}

/// `-var-create {name|"-"} {frame-addr|"*"} expression`
#[derive(Serialize, Default)]
pub struct VarCreate {
    pub positional: Vec<String>,
}
impl VarCreate {
    /// `name = None` lets GDB auto-generate a unique "varNNNNNN" name.
    pub fn new(name: Option<&str>, frame: VarFrame, expression: impl Into<String>) -> Self {
        let frame_str = match frame {
            VarFrame::Current => "*".to_string(),
            VarFrame::Address(a) => a,
        };
        Self {
            positional: vec![
                name.unwrap_or("-").to_string(),
                frame_str,
                expression.into(),
            ],
        }
    }
}
impl MiCommand for VarCreate {
    const OP: &'static str = "var-create";
    type Reply = VarCreateReply;
}
#[derive(Deserialize, Debug)]
pub struct VarCreateReply {
    pub name: String,
    pub numchild: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// `-var-delete name`
#[derive(Serialize, Default)]
pub struct VarDelete {
    pub positional: String,
}
impl MiCommand for VarDelete {
    const OP: &'static str = "var-delete";
    type Reply = EmptyReply;
}

pub enum VarFormat {
    Binary,
    Decimal,
    Hexadecimal,
    Octal,
    Natural,
}
impl VarFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Decimal => "decimal",
            Self::Hexadecimal => "hexadecimal",
            Self::Octal => "octal",
            Self::Natural => "natural",
        }
    }
}

/// `-var-set-format name format-spec`
#[derive(Serialize, Default)]
pub struct VarSetFormat {
    pub positional: Vec<String>,
}
impl VarSetFormat {
    pub fn new(name: impl Into<String>, format: VarFormat) -> Self {
        Self {
            positional: vec![name.into(), format.as_str().to_string()],
        }
    }
}
impl MiCommand for VarSetFormat {
    const OP: &'static str = "var-set-format";
    type Reply = EmptyReply;
}

/// `-var-show-format name`
#[derive(Serialize, Default)]
pub struct VarShowFormat {
    pub positional: String,
}
impl MiCommand for VarShowFormat {
    const OP: &'static str = "var-show-format";
    type Reply = VarShowFormatReply;
}
#[derive(Deserialize, Debug)]
pub struct VarShowFormatReply {
    pub format: String,
}

/// `-var-info-num-children name`
#[derive(Serialize, Default)]
pub struct VarInfoNumChildren {
    pub positional: String,
}
impl MiCommand for VarInfoNumChildren {
    const OP: &'static str = "var-info-num-children";
    type Reply = VarInfoNumChildrenReply;
}
#[derive(Deserialize, Debug)]
pub struct VarInfoNumChildrenReply {
    pub numchild: String,
}

/// `-var-list-children name`
#[derive(Serialize, Default)]
pub struct VarListChildren {
    pub positional: String,
}
impl MiCommand for VarListChildren {
    const OP: &'static str = "var-list-children";
    type Reply = VarListChildrenReply;
}
#[derive(Deserialize, Debug)]
pub struct VarListChildrenReply {
    pub numchild: String,
    pub children: Vec<VarChild>,
}
#[derive(Deserialize, Debug)]
pub struct VarChild {
    pub name: String,
    pub numchild: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// `-var-info-type name`
#[derive(Serialize, Default)]
pub struct VarInfoType {
    pub positional: String,
}
impl MiCommand for VarInfoType {
    const OP: &'static str = "var-info-type";
    type Reply = VarInfoTypeReply;
}
#[derive(Deserialize, Debug)]
pub struct VarInfoTypeReply {
    #[serde(rename = "type")]
    pub ty: String,
}

/// `-var-info-expression name`
#[derive(Serialize, Default)]
pub struct VarInfoExpression {
    pub positional: String,
}
impl MiCommand for VarInfoExpression {
    const OP: &'static str = "var-info-expression";
    type Reply = VarInfoExpressionReply;
}
#[derive(Deserialize, Debug)]
pub struct VarInfoExpressionReply {
    pub lang: String,
    pub exp: String,
}

/// `-var-show-attributes name`
#[derive(Serialize, Default)]
pub struct VarShowAttributes {
    pub positional: String,
}
impl MiCommand for VarShowAttributes {
    const OP: &'static str = "var-show-attributes";
    type Reply = VarShowAttributesReply;
}
#[derive(Deserialize, Debug)]
pub struct VarShowAttributesReply {
    pub status: String,
}

/// `-var-evaluate-expression name`
#[derive(Serialize, Default)]
pub struct VarEvaluateExpression {
    pub positional: String,
}
impl MiCommand for VarEvaluateExpression {
    const OP: &'static str = "var-evaluate-expression";
    type Reply = super::data::ValueReply;
}

/// `-var-assign name expression`
#[derive(Serialize, Default)]
pub struct VarAssign {
    pub positional: Vec<String>,
}
impl VarAssign {
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            positional: vec![name.into(), expression.into()],
        }
    }
}
impl MiCommand for VarAssign {
    const OP: &'static str = "var-assign";
    type Reply = super::data::ValueReply;
}

/// `-var-update {name|"*"}`
#[derive(Serialize, Default)]
pub struct VarUpdate {
    pub positional: String,
}
impl VarUpdate {
    pub fn one(name: impl Into<String>) -> Self {
        Self {
            positional: name.into(),
        }
    }
    pub fn all() -> Self {
        Self {
            positional: "*".to_string(),
        }
    }
}
impl MiCommand for VarUpdate {
    const OP: &'static str = "var-update";
    type Reply = crate::Value;
} // changelist shape not fully specified in this manual version
