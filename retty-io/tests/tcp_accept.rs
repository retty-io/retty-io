use std::net::{IpAddr, SocketAddr};

use retty_io::net::{TcpListener, TcpStream};
#[cfg(unix)]

macro_rules! test_accept {
    ($(($ident:ident, $target:expr),)*) => {
        $(
            #[retty_io::test_all]
            async fn $ident() {
                let listener = TcpListener::bind($target).unwrap();
                let addr = listener.local_addr().unwrap();
                let (tx, rx) = local_sync::oneshot::channel();
                retty_io::spawn(async move {
                    let (socket, _) = listener.accept().await.unwrap();
                    assert!(tx.send(socket).is_ok());
                });
                let cli = TcpStream::connect(&addr).await.unwrap();
                let srv = rx.await.unwrap();
                assert_eq!(cli.local_addr().unwrap(), srv.peer_addr().unwrap());
            }
        )*
    }
}
#[cfg(unix)]

test_accept! {
    (ip_str, "127.0.0.1:0"),
    (host_str, "localhost:0"),
    (socket_addr, "127.0.0.1:0".parse::<SocketAddr>().unwrap()),
    (str_port_tuple, ("127.0.0.1", 0)),
    (ip_port_tuple, ("127.0.0.1".parse::<IpAddr>().unwrap(), 0)),
}
