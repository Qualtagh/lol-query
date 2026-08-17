use super::{Frame, Frames};

#[test]
fn frames_push_pop_frame_mut() {
    let mut frames = Frames::new(2);
    assert_eq!(frames.pop(0), None);
    assert_eq!(frames.pop(1), None);

    frames.push(0, Frame::new(10, 1, true));
    frames.push(0, Frame::new(11, 2, false));
    assert_eq!(frames.frame_mut(0, 10).depth, 1);
    frames.frame_mut(0, 11).active = true;
    assert!(frames.frame_mut(0, 11).active);

    assert_eq!(frames.pop(0), Some(Frame::new(11, 2, true)));
    assert_eq!(frames.pop(0), Some(Frame::new(10, 1, true)));
    assert_eq!(frames.pop(0), None);

    frames.push(1, Frame::new(20, 3, true));
    assert_eq!(frames.peek(1).map(|f| f.instance), Some(20));
    assert_eq!(frames.pop(1), Some(Frame::new(20, 3, true)));
    assert_eq!(frames.peek(1), None);
}
