extern crate tungstenite;
extern crate chrono;

use tungstenite::server::accept;
use chrono::prelude::*;

use std::net::{TcpListener};
use std::thread::spawn;
use std::time::Duration;

mod img;

// A quantification of the differences between two images, based on some specific parameters (see crate::img::calc_diff()).
// If the difference is higher or equal to this number, send the screenshot.
// Min: 0 -> there is no difference at all
// Max: 1000 -> the two images are completely different
static img_diff: u16 = 0; //UPDATE THIS

// Delay between checking screenshots (in ms)
static screen_delay: u64 = 5000;

// Listen for connections on this address
static listen_on: &str = "127.0.0.1:4444";

// Save screenshots on disk (on this path) in case there was a network issue
// Use local directory
static path: String = "".into();

enum MachineKind {
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

    match mkind {
        MachineKind::Unix => {
            println!("Unix-like machine detected. Starting...");
        },
        MachineKind::Windows => {
            println!("Windows-like machine detected. Starting...");
        },
    }

    let server = TcpListener::bind(listen_on).unwrap();
    println!("Listening on {} for connections...", listen_on);

    let mut previous_screenshot: img::RgbaImage;
    let mut current_screenshot: img::RgbaImage;

    // Listen for WebSocket connections
    for stream in server.incoming() {
        spawn (move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {

                // Take a screenshot (and create a filename for it)
                let filename = Local::now().format("Screenshot_%H-%M-%S").to_string();
                let screenshot: img::RgbaImage = img::screenshot_active_window(mkind, filename).expect("An error occurred during the screenshot process (filesystem I/O ?)");

                // Move screenshots
                previous_screenshot = current_screenshot;
                current_screenshot = screenshot;                

                // Calculate the difference between the two images
                let diff = img::calc_diff(previous_screenshot, current_screenshot);

                // If it's huge (aka most of the previous things has been deleted), send previous_screenshot (current_screenshot contains the blank one) as Vec<u8>
                if diff >= img_diff {

                    let image_as_bytevector: img::RgbaImage;

                    // Make image ok for sending (from img::RgbaImage to Vec<u8>)
                    for subpixel in previous_screenshot.data {
                        image_as_bytevector.data.push(subpixel)
                    }
                

                    let time_now = Local::now();

                    println!("[{}:{}:{}] Sending image...", time_now.hour(), time_now.minute(), time_now.second());
                    
                    // Send image
                    websocket.write_message(image_as_bytevector.as_vec_u8().into());
                }

                // Sleep to prevent accidentally DoSsing the bot
                std::thread::sleep(Duration::from_millis(screen_delay))
            }
        });
    }
}
