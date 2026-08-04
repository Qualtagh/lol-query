use std::cell::RefCell;
use std::rc::Rc;

use lol_html::{HtmlRewriter, Settings, comments, doc_comments, doc_text, doctype, element, end, text};

use super::{HandlerEntry, SettingsExt};

type Log = Rc<RefCell<Vec<String>>>;

fn check(handlers: impl FnOnce(Log) -> Vec<HandlerEntry<'static, 'static>>, html: &str, expected: &[&str]) {
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers(log.clone())), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
    assert_eq!(*log.borrow(), expected);
}

#[test]
fn settings_add_handlers() {
    // element! + text! + comments! (element-level handlers)
    check(
        |log| {
            let [l2, l3] = [log.clone(), log.clone()];
            vec![
                element!("p", move |el| {
                    log.borrow_mut().push(el.tag_name());
                    Ok(())
                })
                .into(),
                text!("p", move |t| {
                    let s = t.as_str().to_string();
                    if !s.is_empty() {
                        l2.borrow_mut().push(s);
                    }
                    Ok(())
                })
                .into(),
                comments!("p", move |c| {
                    l3.borrow_mut().push(c.text().to_string());
                    Ok(())
                })
                .into(),
            ]
        },
        r#"<p>hello<!--note--></p><div>ignored</div>"#,
        &["p", "hello", "note"],
    );

    // doctype! + doc_comments! + element! + doc_text! + end! (document-level + element-level)
    check(
        |log| {
            let [l2, l3, l4, l5] = [log.clone(), log.clone(), log.clone(), log.clone()];
            vec![
                doctype!(move |d| {
                    l2.borrow_mut().push(format!("doctype:{}", d.name().unwrap_or_default()));
                    Ok(())
                })
                .into(),
                doc_comments!(move |c| {
                    l3.borrow_mut().push(format!("comment:{}", c.text()));
                    Ok(())
                })
                .into(),
                element!("a", move |el| {
                    log.borrow_mut().push(el.get_attribute("href").unwrap_or_default());
                    Ok(())
                })
                .into(),
                doc_text!(move |t| {
                    let s = t.as_str().to_string();
                    if !s.is_empty() {
                        l4.borrow_mut().push(format!("text:{s}"));
                    }
                    Ok(())
                })
                .into(),
                end!(move |_| {
                    l5.borrow_mut().push("end".to_string());
                    Ok(())
                })
                .into(),
            ]
        },
        r#"<!DOCTYPE html><!-- doc --><a href="url">link</a>"#,
        &["doctype:html", "comment: doc ", "url", "text:link", "end"],
    );
}
