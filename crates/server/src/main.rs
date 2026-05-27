use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

use mio::{Events, Interest, Poll, Token, net::TcpListener};

fn main() {
    let addr: SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("hardcoded address must be valid");
    let mut listener: TcpListener = TcpListener::bind(addr).expect("failed to bind address");

    let _pinger = thread::spawn(|| {
        let domain = "raft-headless.default.svc.cluster.local";
        let pod_ordinal = std::env::var("HOSTNAME").expect("hostname missing in pod");
        let mut peer_addrs: Vec<Ipv4Addr> = Vec::new();

        // sleep 30 seconds
        sleep(Duration::from_secs(30));
        // discover peers
        for i in 0..3 {
            let ordinal = format!("raft-{}", i);
            if ordinal == pod_ordinal {
                continue;
            }
            let hostname = format!("{}.{}", ordinal, domain);
            println!("resolving hostname: {}", hostname);
            match resolve::resolve_host(&hostname) {
                Ok(addrs) => {
                    for ip_addr in addrs {
                        match ip_addr {
                            std::net::IpAddr::V4(addr) => {
                                println!("found ipv4 addr: {} for hostname: {}", addr, hostname);
                                peer_addrs.push(addr);
                            }
                            std::net::IpAddr::V6(addr) => {
                                println!("found ipv6 addr: {} for hostname: {}", addr, hostname);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("resolve failed: {}", e),
            }
        }

        // create poll
        // connect to peers
        let mut streams: Vec<TcpStream> = Vec::new();
        for addr in peer_addrs {
            let sock_addr: SocketAddr = (addr, 8080).into();
            let stream = TcpStream::connect(sock_addr).expect("failed to connect");
            streams.push(stream);
        }

        // ping peers
        loop {
            for stream in streams.iter_mut() {
                stream.write_all(&[1u8]).expect("write failed");
            }
            sleep(Duration::from_secs(5));
        }
    });

    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    poll.registry()
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();

    let mut token_to_stream: HashMap<usize, mio::net::TcpStream> = HashMap::new();

    // loop accept
    //  add to epoll
    //  when readable event -> read to EAGAIN/EWOULDBLOCK
    //  when writable event -> write single byte
    let mut i = 1;
    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                Token(0) => match listener.accept() {
                    Ok((mut stream, _)) => {
                        poll.registry()
                            .register(&mut stream, Token(i), Interest::READABLE)
                            .expect("register failed");
                        token_to_stream.insert(i, stream);
                    }
                    Err(_) => {
                        panic!("failed to accept");
                    }
                },
                Token(i) => {
                    let mut stream = token_to_stream.get(&i).unwrap();
                    let mut buf: [u8; 1024] = [0u8; 1024];
                    let _n = stream.read(&mut buf).unwrap();
                    println!(
                        "received ping message from: {}",
                        stream.peer_addr().unwrap()
                    );
                }
            }
        }

        i += 1;
    }
}
