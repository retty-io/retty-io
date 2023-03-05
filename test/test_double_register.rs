//! A smoke test for windows compatibility

#[test]
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn test_double_register() {
    use retty_io::net::TcpListener;
    use retty_io::*;

    let poll = Poll::new().unwrap();

    // Create the listener
    let l = TcpListener::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();

    // Register the listener with `Poll`
    poll.register(&l, Token(0), Ready::readable(), PollOpt::edge())
        .unwrap();
    assert!(poll
        .register(&l, Token(1), Ready::readable(), PollOpt::edge())
        .is_err());
}
