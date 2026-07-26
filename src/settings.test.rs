use lol_html::{HtmlRewriter, Settings, element};

use super::SettingsExt;

fn push_id(buf: &mut String, id: Option<String>) {
    if let Some(id) = id {
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(&id);
    }
}

#[test]
fn settings_add_handlers_single() {
    let mut ids = String::new();
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(
        Settings::new().add_handlers(vec![element!("div", |el| {
            push_id(&mut ids, el.get_attribute("id"));
            Ok(())
        })
        .into()]),
        |c: &[u8]| out.extend_from_slice(c),
    );
    rw.write(r#"<div id="a"></div><p id="b"></p>"#.as_bytes()).unwrap();
    rw.end().unwrap();
    assert_eq!(ids, "a");
}

#[test]
fn settings_add_handlers_multiple() {
    let mut div_ids = String::new();
    let mut p_ids = String::new();
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(
        Settings::new().add_handlers(vec![
            element!("div", |el| {
                push_id(&mut div_ids, el.get_attribute("id"));
                Ok(())
            })
            .into(),
            element!("p", |el| {
                push_id(&mut p_ids, el.get_attribute("id"));
                Ok(())
            })
            .into(),
        ]),
        |c: &[u8]| out.extend_from_slice(c),
    );
    rw.write(r#"<div id="a"></div><p id="b"></p><div id="c"></div>"#.as_bytes()).unwrap();
    rw.end().unwrap();
    assert_eq!(div_ids, "a c");
    assert_eq!(p_ids, "b");
}
