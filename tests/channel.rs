#[cfg(test)]
mod tests {
    #[test]
    fn test_channel() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = retty_io::channel();

        let handler = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let _ = tx.send("Hello world!");
        });

        let mut handler1 = None;
        let mut handler2 = None;
        for i in 0..2 {
            let mut rx = rx.clone();
            let h = std::thread::spawn(move || {
                const CHANNEL: mio::Token = mio::Token(0);

                let mut poll = mio::Poll::new()?;
                let mut events = mio::Events::with_capacity(2);
                let _ = poll
                    .registry()
                    .register(&mut rx, CHANNEL, mio::Interest::READABLE)?;

                let _ = poll.poll(&mut events, None)?;

                for event in events.iter() {
                    match event.token() {
                        CHANNEL => {
                            println!("receive CHANNEL {}", i);
                            let _ = rx.try_recv();
                        }
                        _ => unreachable!(),
                    }
                }

                Ok::<(), std::io::Error>(())
            });
            if i == 0 {
                handler1 = Some(h);
            } else {
                handler2 = Some(h);
            }
        }
        if let Some(h) = handler1.take() {
            let _ = h.join();
        }
        if let Some(h) = handler2.take() {
            let _ = h.join();
        }
        let _ = handler.join();

        Ok(())
    }
}
