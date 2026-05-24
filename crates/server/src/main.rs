use std::{
    io,
    net::SocketAddr,
    thread::{self, sleep},
    time::Duration,
};

use mio::net::TcpListener;

fn main() {
    let addr: SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("hardcoded address must be valid");
    let listener: TcpListener = TcpListener::bind(addr).expect("failed to bind address");

    let _pinger = thread::spawn(|| {
        let domain = "raft-headless.default.svc.cluster.local";
        let pod_ordinal = std::env::var("HOSTNAME").expect("hostname missing in pod");
        loop {
            sleep(Duration::from_secs(1));
            for i in 0..3 {
                let ordinal = format!("raft-{}", i);
                if ordinal == pod_ordinal {
                    continue;
                }
                let hostname = format!("{}.{}", ordinal, domain);
                println!("resolving hostname: {}", hostname);
                match resolve::resolve_host(&hostname) {
                    Ok(mut addrs) => {
                        let addr = addrs.next().expect("empty resolve host");
                        let n = addrs.count();

                        if n == 0 {
                            println!("\"{}\" resolved to {}", hostname, addr);
                        } else {
                            println!("\"{}\" resolved to addresses: {}", hostname, addr);
                        }
                    }
                    Err(e) => eprintln!("resolve failed: {}", e),
                }
            }
        }
    });

    // loop accept
    //  add to epoll
    //  when readable event -> read to EAGAIN/EWOULDBLOCK
    //  when writable event -> write single byte
    loop {
        match listener.accept() {
            Ok((_, addr)) => {
                println!("accepted addr: {}", addr);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                panic!("accept failed: {}", e);
            }
        }
    }
}
