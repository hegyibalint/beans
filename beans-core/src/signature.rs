#[derive(Debug, Clone, PartialEq)]
pub enum Signature {
    Method {
        return_type: String,
        parameters: Vec<MethodParam>,
        type_parameters: Vec<String>,
    },
    Field {
        field_type: String,
    },
    Class {
        type_parameters: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodParam {
    pub name: String,
    pub param_type: String,
}
