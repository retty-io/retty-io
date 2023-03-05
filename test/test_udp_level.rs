use retty_io::event::Event;
use retty_io::net::UdpSocket;
use retty_io::{Events, Poll, PollOpt, Ready, Token};
use std::time::{Duration, Instant};
use {expect_events, sleep_ms};

#[test]
pub fn test_udp_level_triggered() {
    let poll = Poll::new().unwrap();
    let poll = &poll;
    let mut events = Events::with_capacity(1024);
    let events = &mut events;

    // Create the listener
    let tx = UdpSocket::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
    let rx = UdpSocket::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();

    poll.register(
        &tx,
        Token(0),
        Ready::readable() | Ready::writable(),
        PollOpt::level(),
    )
    .unwrap();
    poll.register(
        &rx,
        Token(1),
        Ready::readable() | Ready::writable(),
        PollOpt::level(),
    )
    .unwrap();

    for _ in 0..2 {
        expect_events(
            poll,
            events,
            2,
            vec![
                Event::new(Ready::writable(), Token(0)),
                Event::new(Ready::writable(), Token(1)),
            ],
        );
    }

    tx.send_to(b"hello world!", &rx.local_addr().unwrap())
        .unwrap();

    sleep_ms(250);

    for _ in 0..2 {
        expect_events(
            poll,
            events,
            2,
            vec![Event::new(Ready::readable() | Ready::writable(), Token(1))],
        );
    }

    let mut buf = [0; 200];
    while rx.recv_from(&mut buf).is_ok() {}

    for _ in 0..2 {
        expect_events(
            poll,
            events,
            4,
            vec![Event::new(Ready::writable(), Token(1))],
        );
    }

    tx.send_to(b"hello world!", &rx.local_addr().unwrap())
        .unwrap();
    sleep_ms(250);

    expect_events(
        poll,
        events,
        10,
        vec![Event::new(Ready::readable() | Ready::writable(), Token(1))],
    );

    drop(rx);
}

#[test]
pub fn test_ecn() {
    let socket1 = UdpSocket::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
    let socket2 = UdpSocket::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
    let addr2 = socket2.local_addr().unwrap();

    let contents = (12324343 as u64).to_be_bytes().to_vec();

    let (tx, rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let mut buffers = vec![0, 0, 0, 0, 0, 0, 0, 0];

        const SOCKET_RD: Token = Token(0);
        let poll = Poll::new()?;
        let mut events = Events::with_capacity(2);
        poll.register(&socket2, SOCKET_RD, Ready::readable(), PollOpt::edge())
            .unwrap();
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            match event.token() {
                SOCKET_RD => {
                    let n = socket2.recv(&mut buffers).unwrap();
                    println!("received {} {:?}", n, buffers);
                }
                _ => unreachable!(),
            }
        }
        println!("sending {:?}", buffers);
        let _ = tx.send(Some(buffers));
        println!("sent");

        Ok::<(), std::io::Error>(())
    });

    let start = Instant::now();

    std::thread::sleep(Duration::from_millis(500));

    const SOCKET_WT: Token = Token(0);
    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(2);
    poll.register(&socket1, SOCKET_WT, Ready::writable(), PollOpt::edge())
        .unwrap();
    poll.poll(&mut events, None).unwrap();
    for event in events.iter() {
        match event.token() {
            SOCKET_WT => {
                let n = socket1.send_to(&contents, &addr2).unwrap();
                println!("sent {} packets in {}ms", n, start.elapsed().as_millis());
            }
            _ => unreachable!(),
        }
    }

    let _ = handle.join();

    println!("receiving ecn");
    let ecn = rx.recv().unwrap();
    println!("receiving {:?}", ecn);
    assert!(ecn.is_some());
}
