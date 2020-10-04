extern crate tungstenite;
extern crate chrono;

use tungstenite::server::accept;
use chrono::prelude::*;

use std::net::{TcpListener};
use std::thread::spawn;
use std::time::Duration;

mod img;

// A quantification of the differences between two screenshots (see crate::img::calc_diff()).
// If the difference is higher or equal to this number, send the screenshot.
// Min: 0 -> there is no difference at all
// Max: 100 -> the two images are completely different
static IMG_DIFF: u8 = 0; //UPDATE THIS

// Delay between checking screenshots (in ms)
static SCREEN_DELAY: u64 = 5000;

// Listen for connections on this address
static LISTEN_ON: &str = "127.0.0.1:4444";

// Save screenshots on disk (on this path) in case there was a network issue
// Use local directory
static SAVE_PATH: &str = "screenshots/";

#[derive(Copy, Clone)]
pub enum MachineKind {
    Unix,
    Windows
}

// Get machine kind (Unix-like/Windows-like/Unknown)
pub fn get_machine_kind() -> Result<MachineKind, ()> {
    if cfg!(unix) {
        Ok(MachineKind::Unix)
    } else if cfg!(windows) {
        Ok(MachineKind::Windows)
    } else {
        Err(())
    }
}

fn main() {

    // Get which kind of machine we're running on
    let mkind = get_machine_kind().expect("Unknown machine kind. Exiting...");

    if let Ok(_) = std::fs::create_dir("screenshots") {
        println!("screenshots dir created.")
    } else {
        println!("Cannot create screenshots dir. It may already exist.")
    }
    
    match mkind {
        MachineKind::Unix => {
            println!("Unix-like machine detected. Starting...");
        },
        MachineKind::Windows => {
            println!("Windows-like machine detected. Starting...");
        },
    }

    loop {

        let server = TcpListener::bind(LISTEN_ON).unwrap();
        println!("Listening on {} for connections...", LISTEN_ON);

        // Listen for WebSocket connections
        for stream in server.incoming() {

            spawn ( move || {

                let stream = stream.unwrap();

                // Remote address of client
                let remote_addr = stream.peer_addr().unwrap();

                let mut websocket = accept(stream).unwrap();
                println!("Connected to {}", remote_addr.clone());


                let mut previous_screenshot: img::RgbImage;
                let mut current_screenshot: img::RgbImage = img::RgbImage::new();

                loop {

                    // Take a screenshot (and create a filename for it)
                    let filename = Local::now().format("Screenshot_%H-%M-%S.png").to_string();
                    let screenshot: img::RgbImage = img::screenshot_active_window(mkind, format!("{}{}", SAVE_PATH, filename)).expect("An error occurred during the screenshot process (filesystem I/O ?)");
                    println!("Screenshot taken!");

                    // Move screenshots
                    previous_screenshot = current_screenshot.clone();
                    current_screenshot = screenshot;                

                    // Calculate the difference between the two images
                    let diff = img::calc_diff(previous_screenshot.clone(), current_screenshot.clone());
                    print!("diff is {}", diff);

                    // If it's huge (aka most of the previous things has been deleted), send previous_screenshot (current_screenshot contains the blank one) as Vec<u8>
                    if diff >= IMG_DIFF {

                        println!(" which is greater than IMG_DIFF ({}).", IMG_DIFF);

                        let time_now = Local::now();

                        println!("[{}:{}:{}] Sending image...", time_now.hour(), time_now.minute(), time_now.second());

                        // Create the message to send
                        let mut msg: Vec<u8> = (previous_screenshot.height.to_string()).as_bytes().to_owned(); // Start with the image height
                        msg.push('|' as u8); // Insert a separator
                        msg.append(&mut previous_screenshot.as_vec_u8().clone()); // Join it with the image data
                        
                        // Send image
                        websocket.write_message(msg.into()).expect("Uncaught WebSocket error");
                    } else {
                        println!(" which is not greater than IMG_DIFF ({})", IMG_DIFF);
                    }

                    // Sleep to prevent accidentally DoSsing the bot
                    std::thread::sleep(Duration::from_millis(SCREEN_DELAY))
                }
            });
        }

        println!("Disconnected");
    }
}
