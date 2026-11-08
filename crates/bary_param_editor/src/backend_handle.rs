use std::net::{SocketAddrV4, UdpSocket};

pub struct BackendHandle {
    socket: UdpSocket,
}

impl BackendHandle {
    pub fn new() -> Self {
        let addr: SocketAddrV4 = "0.0.0.0:5000".parse().unwrap();
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.connect(addr).unwrap();
        socket.set_nonblocking(true).unwrap();

        Self { socket }
    }

    pub fn update(&self) {
        let mut buf = [0; 4096];
        while let Ok(n) = self.socket.recv(&mut buf) {
            if n == 0 {
                break;
            }

            let data = buf[..n].to_vec();
            println!("{} bytes", data.len());
        }
    }
}
