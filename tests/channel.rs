#[cfg(test)]
mod tests {
    use crossbeam::sync::WaitGroup;

    #[test]
    fn test_channel() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = retty_io::channel();

        let handler = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let _ = tx.send("Hello world!");
        });

        let wg = WaitGroup::new();
        for i in 0..2 {
            let mut rx = rx.clone();
            let wg = wg.clone();
            std::thread::spawn(move || {
                const CHANNEL: mio::Token = mio::Token(0);

                let mut poll = mio::Poll::new()?;
                let mut events = mio::Events::with_capacity(2);
                poll.registry()
                    .register(&mut rx, CHANNEL, mio::Interest::READABLE)?;

                poll.poll(&mut events, None)?;

                for event in events.iter() {
                    match event.token() {
                        CHANNEL => {
                            println!("receive CHANNEL {}", i);
                            let _ = rx.try_recv();
                            drop(wg);
                            return Ok(());
                        }
                        _ => unreachable!(),
                    }
                }

                Ok::<(), std::io::Error>(())
            });
        }

        wg.wait();
        let _ = handler.join();

        Ok(())
    }
}
