extern crate chrono;

use chrono::prelude::*;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;
use std::time::Duration;
use std::sync::{Arc, Mutex};

mod img;

// See this (to keep taking screenshots of the same window): https://stackoverflow.com/questions/5262413/does-xlib-have-an-active-window-event

// A quantification of the differences between two screenshots (see crate::img::calc_diff()).
// If the difference is higher or equal to this number, send the screenshot.
// Min: 0 -> there is no difference at all
// Max: DIFF_TOTAL -> the two images are completely different
static IMG_DIFF: f32 = 0.0; //UPDATE THIS
static DIFF_TOTAL: f32 = 100.0;

// Delay between checking screenshots (in ms), the number must be equal or greater than the bot's requests delay
static SCREEN_DELAY: u64 = 2000;

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

// Take the screenshots and return the updated input values
fn screenshooter(previous_scrn: &mut img::RgbImage, current_scrn: &mut img::RgbImage, must_send_prev_scrn: &mut bool, machine_kind: MachineKind) {

    // Take a screenshot (and create a filename for it)
    let filename = Local::now().format("Screenshot_%H-%M-%S.png").to_string();
    let screenshot: img::RgbImage = img::screenshot_active_window(machine_kind, format!("{}{}", SAVE_PATH, filename)).expect("An error occurred during the screenshot process (filesystem I/O ?)");
    println!("Screenshot taken!");

    // Move screenshots
    *previous_scrn = current_scrn.clone();
    *current_scrn = screenshot;

    // Both screenshots cannot be empty to calculate the difference
    if previous_scrn.data.len() != 0 {

        // Calculate the difference between the two image
        let diff: f32 = {
            if current_scrn.data.len() == 0 || previous_scrn.data.len() != current_scrn.data.len() {
                DIFF_TOTAL
            } else {
                img::calc_diff(previous_scrn.clone(), current_scrn.clone())
            }
        };

        // If the difference is huge (aka most of the previous things has been deleted), send previous_screenshot (current_screenshot contains the new blank one)
        if diff >= IMG_DIFF {
            if previous_scrn.height != 0 && previous_scrn.width != 0 {
                println!("    diff = {}, diff >= {} (IMG_DIFF)", diff, IMG_DIFF);
                
                *must_send_prev_scrn = true;

            } else {
                *must_send_prev_scrn = false;
            }
        } else {
            println!("    diff = {}, diff < {} (IMG_DIFF)", diff, IMG_DIFF);
            *must_send_prev_scrn = false;

            // If there are NO differences, also delete the image
            if diff == 0.0 {
                if let Err(_) = std::fs::remove_dir(format!("screenshots/{}", filename)) {
                    println!("Warning: an error occurred while trying to delete a duplicated screenshot (diff = 0).")
                }
            }
        }
    } else {
        println!("The screenshot is empty (length = 0), not going to send it.");
        *must_send_prev_scrn = false;
    }
}

fn handle_client_conn(stream: &mut TcpStream, screenshot: img::RgbImage, must_send_scrn: bool, last_sent_scrn_hash: String) {

    // Remote address of client
    let remote_addr = stream.peer_addr().unwrap();
    println!("Connected to {}", remote_addr.clone());

    // Screenshot's hash
    let scrn_hash = format!("{:x}", md5::compute(screenshot.as_vec_u8()));

    // If must_send_scrn is true and the screenshot hasn't been sent yet, send it 
    if must_send_scrn && last_sent_scrn_hash != scrn_hash {

        // Assert that the screenshot's data is not empty
        assert!(screenshot.data.len() != 0, "previous_screenshot's data doesn't exist.");

        let time_now = Local::now();
        println!("[{}] Sending image ({} x {})...", time_now.format("%H:%M:%S").to_string(), screenshot.width, screenshot.height);

        let img_bytes_num = screenshot.data.len() * 3;

        // Create the message to send
        let mut msg: Vec<u8> = (screenshot.height.to_string()).as_bytes().to_owned(); // Start with the image height
        msg.push('|' as u8); // Insert a separator
        msg.append(&mut img_bytes_num.to_string().as_bytes().to_owned()); // Insert the image length (width * height * 3) in bytes
        msg.push('|' as u8); // Insert a separator
        msg.append(&mut screenshot.as_vec_u8().clone()); // Join it with the image data

        // Send image
        stream.write(&msg).expect("Uncaught stream error");
        println!("  Image sent!");
    } else {

        // Otherwise, send an empty message, meaning that the required screenshot has already been sent
        stream.write(&[]).expect("Uncaught stream error");
    }

    println!("Disconnected from {}", remote_addr.clone());
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

    // Screenshots
    let previous_screenshot: Arc<Mutex<img::RgbImage>> = Arc::new(Mutex::new(img::RgbImage::new()));
    let current_screenshot: Arc<Mutex<img::RgbImage>> = Arc::new(Mutex::new(img::RgbImage::new()));

    // Does previous_screenshot need to be sent? (based on diff)
    let must_send_prev_scrn: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    // MD5 hash of the last sent screenshot (to check if it previous_screenshot has already been sent)
    let last_sent_screenshot_hash: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    {
        let previous_screenshot = previous_screenshot.clone();
        let must_send_prev_scrn = must_send_prev_scrn.clone();
        let must_send_prev_scrn = must_send_prev_scrn.clone();

        // Do screenshots and save them
        std::thread::spawn(move || {
            let mut previous_screenshot = previous_screenshot.lock().unwrap();
            let mut current_screenshot = current_screenshot.lock().unwrap();
            let mut must_send_prev_scrn = must_send_prev_scrn.lock().unwrap();
            
            loop {

                screenshooter(&mut *previous_screenshot, &mut *current_screenshot, &mut *must_send_prev_scrn, mkind);
        
                // Sleep to prevent accidentally DoSsing the bot
                std::thread::sleep(Duration::from_millis(SCREEN_DELAY));
            }
        });
    }

    // Listen for connections
    let listener = TcpListener::bind(LISTEN_ON).expect(&format!("Address {} is already in use.", LISTEN_ON));
    println!("Listening on {} for connections...", LISTEN_ON);

    // Accept connections
    for stream in listener.incoming() {

        let previous_screenshot = previous_screenshot.clone();
        let must_send_prev_scrn = must_send_prev_scrn.clone();
        let last_sent_screenshot_hash = last_sent_screenshot_hash.clone();

        spawn ( move || {

            let previous_screenshot = previous_screenshot.lock().unwrap();
            let must_send_prev_scrn = must_send_prev_scrn.lock().unwrap();
            let last_sent_screenshot_hash = last_sent_screenshot_hash.lock().unwrap();

            handle_client_conn(&mut stream.expect("Stream error"), (*previous_screenshot).clone(), *must_send_prev_scrn, (*last_sent_screenshot_hash).clone());
        });
    }
}
