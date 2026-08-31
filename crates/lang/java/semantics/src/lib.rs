use beans_lang_java_model as model;

pub fn lower_into(_content: &str) -> model::File {
    let tree = parser::parse(_content);
    let model = File::new();

    
}
