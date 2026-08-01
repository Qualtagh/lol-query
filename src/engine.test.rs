use std::cell::RefCell;
use std::rc::Rc;

use lol_html::{HtmlRewriter, Settings};

use super::{Engine, InstanceId, Step};
use crate::SettingsExt;

type Log = Rc<RefCell<Vec<String>>>;

fn log() -> Log {
    Rc::new(RefCell::new(vec![]))
}

fn css(selector: &str) -> Vec<Step> {
    vec![Step::Filter(selector.into(), None)]
}

fn exec(handlers: Vec<crate::HandlerEntry<'static, 'static>>, html: &str) {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
}

fn check_match(steps: Vec<Step>, html: &str, expected: &str) {
    let ids = log();
    let ids2 = ids.clone();
    let mut engine = Engine::new();
    let chain = engine.add_chain(steps);
    engine.on_match(chain, move |el| ids2.borrow_mut().push(el.get_attribute("id").unwrap_or_default()));
    exec(engine.into_handlers(), html);
    assert_eq!(ids.borrow().join(" "), expected);
}

fn check_enter_exit(steps: Vec<Step>, html: &str, expected: &str) {
    let events = log();
    let enter_log = events.clone();
    let exit_log = events.clone();
    let mut engine = Engine::new();
    let chain = engine.add_chain(steps);
    engine.on_enter(chain, move |id: InstanceId, _depth, el| {
        enter_log.borrow_mut().push(format!("+{id}:{}", el.get_attribute("id").unwrap_or_default()));
    });
    engine.on_exit(chain, move |id: InstanceId| {
        exit_log.borrow_mut().push(format!("-{id}"));
    });
    exec(engine.into_handlers(), html);
    assert_eq!(events.borrow().join(" "), expected);
}

fn check_two_chains(left: Vec<Step>, right: Vec<Step>, html: &str, expected_left: &str, expected_right: &str) {
    let left_ids = log();
    let right_ids = log();
    let left2 = left_ids.clone();
    let right2 = right_ids.clone();
    let mut engine = Engine::new();
    let left_chain = engine.add_chain(left);
    let right_chain = engine.add_chain(right);
    engine.on_match(left_chain, move |el| left2.borrow_mut().push(el.get_attribute("id").unwrap_or_default()));
    engine.on_match(right_chain, move |el| right2.borrow_mut().push(el.get_attribute("id").unwrap_or_default()));
    exec(engine.into_handlers(), html);
    assert_eq!(left_ids.borrow().join(" "), expected_left);
    assert_eq!(right_ids.borrow().join(" "), expected_right);
}

#[test]
fn engine() {
    // on_match — single chain still matches through the extracted engine
    check_match(css("div"), r#"<div id="a"></div><span id="b"></span>"#, "a");

    // on_enter / on_exit — nested matches get distinct ids; exits are LIFO
    check_enter_exit(css("div"), r#"<div id="a"><div id="b"></div></div>"#, "+0:a +1:b -1 -0");

    // on_enter / on_exit — void elements enter and exit immediately
    check_enter_exit(css("img"), r#"<img id="a"><span id="b"></span>"#, "+0:a -0");

    // two chains on one Engine — one combine pass, independent callbacks
    check_two_chains(css("div"), css("span"), r#"<div id="a"></div><span id="b"></span><div id="c"><span id="d"></span></div>"#, "a c", "b d");

    // two chains — an element matched by both is reported on both
    check_two_chains(css("div"), css(".item"), r#"<div class="item" id="a"></div><div id="b"></div>"#, "a b", "a");
}
