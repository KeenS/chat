extern crate mio;

use self::mio::*;
use self::mio::tcp::*;
use std::collections::HashMap;
use super::server::WebSocktServer;

pub struct WebSocktHandler {
    pub socket: TcpListener,
    pub clients: HashMap<Token, WebSocktServer>,
    token_counter: usize
}

const SEVER_TOKEN: Token = Token(0);

impl WebSocktHandler {
    pub fn new(listener: TcpListener) -> WebSocktHandler {
        WebSocktHandler {
            token_counter: 1,
            clients: HashMap::new(),
            socket: listener
        }
    }
}

impl Handler for WebSocktHandler {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<WebSocktHandler>,
             token: Token, events: EventSet) {
        if events.is_readable() {
            match token {
                SEVER_TOKEN => {
                    let client_socket = match self.socket.accept() {
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        },
                        Ok(None) => panic!("Accept has returned 'None'"),
                        Ok(Some(sock)) => sock
                    };
                    self.token_counter += 1;
                    let new_token = Token(self.token_counter);
                    self.clients.insert(new_token, WebSocktServer::new(client_socket));
                    event_loop.register_opt(&self.clients[&new_token].socket,
                                            new_token, EventSet::readable(),
                                            PollOpt::edge() | PollOpt::oneshot()).unwrap();
                },
                token => {
                    let mut client = self.clients.get_mut(&token).unwrap();
                    client.read();
                    event_loop.reregister(&client.socket, token, client.interest,
                                          PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
            }
        }
        if events.is_writable() {
            let mut client = self.clients.get_mut(&token).unwrap();
            client.write();
            event_loop.reregister(&client.socket, token, client.interest,
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
        }
    }
}
