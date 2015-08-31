extern crate mio;
extern crate chat;

use mio::*;
use mio::tcp::*;
use chat::server::WebSocketServer;

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let server_socket = TcpSocket::v4().unwrap();
    let address = std::str::FromStr::from_str("0.0.0.0:10000").unwrap();
    server_socket.bind(&address).unwrap();
    let listener = server_socket.listen(128).unwrap();
    let mut server = WebSocketServer::new(listener);
    event_loop.register_opt(&server.socket,
                            Token(0),
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();
    event_loop.run(&mut server).unwrap();
}
