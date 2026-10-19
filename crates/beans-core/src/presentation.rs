pub struct RichText {
    plaintext: String,
    markdown: Option<String>,
}

impl RichText {
    pub fn new(plaintext: impl Into<String>) -> Self {
        Self {
            plaintext: plaintext.into(),
            markdown: None,
        }
    }

    pub fn with_markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown = Some(markdown.into());
        self
    }

    pub fn plaintext(&self) -> &str {
        &self.plaintext
    }

    pub fn markdown(&self) -> Option<&str> {
        self.markdown.as_deref()
    }
}
