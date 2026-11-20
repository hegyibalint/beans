use lsp_server::{ErrorCode, Request, Response};
use lsp_types::Uri;
use serde_json::json;

use super::Server;

pub(super) const METHOD: &str = "workspace/textDocumentContent";
pub(super) const JVM_SCHEME: &str = "beans+jvm";

impl Server {
    pub(super) fn text_document_content(&self, request: Request) -> Response {
        let Some(name) = request
            .params
            .get("uri")
            .and_then(|uri| uri.as_str())
            .and_then(|uri| uri.parse::<Uri>().ok())
            .and_then(|uri| jvm_class_name(&uri))
        else {
            return Response::new_err(
                request.id,
                ErrorCode::InvalidParams as i32,
                "Expected params.uri to be a JVM document URI like beans+jvm:/java/lang/String.class".into(),
            );
        };

        match self.features.as_ref().unwrap().preview_jvm_class(&name) {
            Ok(text) => Response::new_ok(request.id, json!({ "text": text })),
            Err(error) => Response::new_err(request.id, ErrorCode::RequestFailed as i32, error),
        }
    }
}

fn jvm_class_name(uri: &Uri) -> Option<String> {
    let path = uri.path();
    if !uri.scheme()?.as_str().eq_ignore_ascii_case(JVM_SCHEME)
        || uri.authority().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || !path.is_absolute()
    {
        return None;
    }

    // Decode each URI segment separately; an escaped slash isn't a path separator.
    let mut segments = path.segments().peekable();
    let mut name = String::new();
    while let Some(segment) = segments.next() {
        let decoded = segment.decode().into_string().ok()?;
        let component = if segments.peek().is_none() {
            decoded.strip_suffix(".class")?
        } else {
            &decoded
        };
        // Internal JVM names consist of slash-separated unqualified names (JVMS §4.2).
        if component.is_empty() || component.contains(['.', ';', '[', '/']) {
            return None;
        }
        if !name.is_empty() {
            name.push('/');
        }
        name.push_str(component);
    }
    (!name.is_empty()).then_some(name)
}
