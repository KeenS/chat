extern crate mio;
extern crate http_muncher;
extern crate sha1;
extern crate rustc_serialize;
extern crate std;

use self::mio::*;
use self::mio::tcp::*;
use self::http_muncher::{Parser, ParserHandler};
use self::rustc_serialize::base64::{ToBase64, STANDARD};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;
use std::str;
use std::collections::VecDeque;
use super::frame;

fn gen_key(key: &String) -> String {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];
    m.update(key.as_bytes());
    m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());
    m.output(&mut buf);

    return buf.to_base64(STANDARD);
}




struct HttpParser {
    current_key: Option<String>,
    headers: Rc<RefCell<HashMap<String, String>>>
}
impl ParserHandler for HttpParser{
    fn on_header_field(&mut self, s: &[u8]) -> bool {
        self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, s: &[u8]) -> bool {
        self.headers.borrow_mut().insert(self.current_key.clone().unwrap(),
                            std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_headers_complete(&mut self) -> bool {
        false
    }
}


pub struct WebSocktServer {
    pub socket: TcpStream,
    http_parser: Parser<HttpParser>,
    headers: Rc<RefCell<HashMap<String, String>>>,
    pub interest: EventSet,
    state: ClientState,
    message_buffer: VecDeque<Vec<u8>>
}

#[derive(Debug)]
enum ClientState {
    AwaitingHandshake,
    HandshakeResponse,
    Connected
}

impl WebSocktServer {
    pub fn new(socket: TcpStream) -> WebSocktServer {
        let headers = Rc::new(RefCell::new(HashMap::new()));
        WebSocktServer {
            headers: headers.clone(),
            socket: socket,
            http_parser: Parser::request(HttpParser{
                current_key: None,
                headers: headers.clone()
            }),
            interest: EventSet::readable(),
            state: ClientState::AwaitingHandshake,
            message_buffer: VecDeque::new()
        }
    }

    pub fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf) {
                Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    return
                },
                Ok(None) => break,
                Ok(Some(len)) => {
                    println!("{:?}", &buf[0..]);
                    match self.state {
                        ClientState::AwaitingHandshake => {
                            self.http_parser.parse(&buf[0..len]);
                            if self.http_parser.is_upgrade() {
                                self.state = ClientState::HandshakeResponse;
                                self.interest.remove(EventSet::readable());
                                self.interest.insert(EventSet::writable());
                                break;
                            }
                        },
                        ClientState::Connected => {
                            let (opcode, msg) = frame::parse_frame(&buf[0..len]).unwrap();
                            println!("{:?}", opcode);
                            match opcode {
                                frame::Opcode::Text => {
                                    self.message_buffer.push_back(msg);
                                    self.interest.remove(EventSet::readable());
                                    self.interest.insert(EventSet::writable());

                                },
                                frame::Opcode::Binary => println!("{:?}", msg),
                                _ => ()
                            }
                            break;
                        },
                        _  => break
                    }
                }
            }
        }
    }

    pub fn write(&mut self) {
        match self.state {
            ClientState::HandshakeResponse => {
                let headers = self.headers.borrow();
                let response_key = gen_key(&headers.get("Sec-WebSocket-Key").unwrap());
                let response = fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                                         Connection: Upgrade\r\n\
                                                         Sec-WebSocket-Accept: {}\r\n\
                                                         Upgrade: websocket\r\n\r\n", response_key));
                self.socket.try_write(response.as_bytes()).unwrap();
                self.state = ClientState::Connected;
                self.interest.remove(EventSet::writable());
                self.interest.insert(EventSet::readable());
                
            }
            ClientState::Connected => {
                match self.message_buffer.pop_front() {
                    None => return,
                    Some(msg) => {
                        let frame = frame::pack_message(&msg[..], None).unwrap();
                        self.socket.try_write(&frame[..]).unwrap();
                        if self.message_buffer.is_empty() {
                            self.interest.remove(EventSet::writable());                            
                        }
                        self.interest.insert(EventSet::readable());

                    }
                }
            },
            _ => return
        }
    }
}

