use super::{Cursor, Span, Template};

impl Template {
    /// Removes fixture markers and records their byte positions in the resulting content.
    /// Unknown angle-bracket text is left alone; malformed fixture markers panic.
    pub fn parse(source: &str) -> Self {
        let mut content = String::with_capacity(source.len());
        let mut spans: Vec<Span> = Vec::new();
        let mut cursors: Vec<Cursor> = Vec::new();
        let mut open_spans: Vec<usize> = Vec::new();
        let mut rest = source;

        while let Some(at) = rest.find('<') {
            content.push_str(&rest[..at]);
            let tail = &rest[at..];
            let Some(close) = tail.find('>') else {
                assert!(
                    !tail.starts_with("<cur:")
                        && !tail.starts_with("<span:")
                        && !tail.starts_with("</span"),
                    "unclosed fixture marker in {tail:?}"
                );
                content.push('<');
                rest = &tail[1..];
                continue;
            };
            let tag = &tail[..=close];

            match tag {
                "<cur>" => cursors.push(Cursor {
                    name: None,
                    offset: content.len(),
                }),
                "<span>" => {
                    open_spans.push(spans.len());
                    spans.push(Span {
                        name: None,
                        start: content.len(),
                        end: content.len(),
                    });
                }
                "</span>" => {
                    let index = open_spans
                        .pop()
                        .expect("closing span without an opening span");
                    spans[index].end = content.len();
                }
                _ if tag.starts_with("<cur:") => {
                    let name = marker_name(tag, "<cur:");
                    assert!(
                        !cursors
                            .iter()
                            .any(|cursor| cursor.name.as_deref() == Some(name)),
                        "duplicate cursor name {name:?}"
                    );
                    cursors.push(Cursor {
                        name: Some(name.into()),
                        offset: content.len(),
                    });
                }
                _ if tag.starts_with("<span:") => {
                    let name = marker_name(tag, "<span:");
                    assert!(
                        !spans.iter().any(|span| span.name.as_deref() == Some(name)),
                        "duplicate span name {name:?}"
                    );
                    open_spans.push(spans.len());
                    spans.push(Span {
                        name: Some(name.into()),
                        start: content.len(),
                        end: content.len(),
                    });
                }
                _ if tag.starts_with("</span") => panic!("invalid span closing tag {tag:?}"),
                _ => {
                    // Preserve unfamiliar angle-bracket text, including Java generics.
                    // Consume only '<' so a later marker is still recognized.
                    content.push('<');
                    rest = &tail[1..];
                    continue;
                }
            }
            rest = &tail[close + 1..];
        }
        content.push_str(rest);
        assert!(open_spans.is_empty(), "span without a closing </span>");

        Self {
            content,
            spans,
            cursors,
        }
    }
}

fn marker_name<'a>(tag: &'a str, prefix: &str) -> &'a str {
    let name = tag.strip_prefix(prefix).unwrap().strip_suffix('>').unwrap();
    assert!(
        !name.is_empty()
            && !name
                .chars()
                .any(|character| character.is_whitespace() || matches!(character, '<' | '/')),
        "invalid fixture marker {tag:?}"
    );
    name
}

#[cfg(test)]
mod tests {
    use super::Template;

    #[test]
    fn standalone_cursors_point_into_clean_content() {
        let template = Template::parse("é<cur> a<cur:second>b");

        assert_eq!(template.content, "é ab");
        assert_eq!(template.cursors[0].name, None);
        assert_eq!(template.cursors[0].offset, 2);
        assert_eq!(template.cursors[1].name.as_deref(), Some("second"));
        assert_eq!(template.cursors[1].offset, 4);
    }

    #[test]
    fn paired_spans_and_cursors_can_share_a_name_without_being_linked() {
        let template = Template::parse("<span:name>as<cur:name>dasd</span>");

        assert_eq!(template.content, "asdasd");
        assert_eq!(template.spans[0].name.as_deref(), Some("name"));
        assert_eq!((template.spans[0].start, template.spans[0].end), (0, 6));
        assert_eq!(template.cursors[0].offset, 2);
        assert_eq!(
            template.spans[0]
                .positions(&template.content)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn nested_and_empty_spans_keep_their_opening_order() {
        let template = Template::parse("x<span:outer>a<span>b</span>c</span><span></span>z");

        assert_eq!(template.content, "xabcz");
        assert_eq!((template.spans[0].start, template.spans[0].end), (1, 4));
        assert_eq!((template.spans[1].start, template.spans[1].end), (2, 3));
        assert_eq!((template.spans[2].start, template.spans[2].end), (4, 4));
    }

    #[test]
    fn unrelated_angle_brackets_are_not_markers() {
        let template = Template::parse("List<String> x = a < b ? <cur>c : d;");

        assert_eq!(template.content, "List<String> x = a < b ? c : d;");
        assert_eq!(
            template.cursors[0].offset,
            template.content.find("c : d").unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "closing span without an opening span")]
    fn unmatched_closing_span_is_rejected() {
        Template::parse("</span>");
    }

    #[test]
    #[should_panic(expected = "span without a closing")]
    fn unclosed_span_is_rejected() {
        Template::parse("<span:name>text");
    }

    #[test]
    #[should_panic(expected = "duplicate cursor name")]
    fn duplicate_named_cursors_are_rejected() {
        Template::parse("<cur:name>a<cur:name>");
    }

    #[test]
    #[should_panic(expected = "duplicate span name")]
    fn duplicate_named_spans_are_rejected() {
        Template::parse("<span:name>a</span><span:name>b</span>");
    }

    #[test]
    #[should_panic(expected = "invalid fixture marker")]
    fn empty_names_are_rejected() {
        Template::parse("<cur:>");
    }
}
