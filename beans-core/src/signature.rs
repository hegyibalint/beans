use crate::type_ref::{TypeParam, TypeRef};

#[derive(Debug, Clone, PartialEq)]
pub enum Signature {
    Method {
        return_type: TypeRef,
        parameters: Vec<MethodParam>,
        type_parameters: Vec<TypeParam>,
        throws: Vec<TypeRef>,
    },
    Field {
        field_type: TypeRef,
        constant_value: Option<ConstantValue>,
        initialized: bool,
    },
    Class {
        type_parameters: Vec<TypeParam>,
    },
    Record {
        type_parameters: Vec<TypeParam>,
        components: Vec<RecordComponent>,
    },
    AnnotationElement {
        element_type: TypeRef,
        default_value: Option<ConstantValue>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodParam {
    pub name: String,
    pub param_type: TypeRef,
    pub is_varargs: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordComponent {
    pub name: String,
    pub component_type: TypeRef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Null,
}
