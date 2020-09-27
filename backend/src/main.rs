extern crate tungstenite;

use tungstenite::server::accept;

use std::net::{TcpListener};
use std::thread::spawn;

fn main() {
    let server = TcpListener::bind("127.0.0.1:4444").unwrap();

    //let mut previous_screenshot: Vec<Vec<u8>> = vec![];
    //let mut current_screenshot: Vec<Vec<u8>> = vec![];

    // Listen for WebSocket connections
    for stream in server.incoming() {
        spawn (move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                // Take a screenshot

                // Put the content of current_screenshot in previous_screenshot

                // Calculate the difference between the two

                // If it's huge (aka the previous things has been deleted), send the previous_screenshot (current_screenshot contains the blank one)


                //websocket.write_message("".into());
            }
        });
    }
}
