extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate xdotool;
extern crate regex;

use chrono::prelude::*;
use xdotool::{
    command::{
        Command,
        sub_commands::Window::{
            SelectWindow,
        },
    },
    optionvec::OptionVec,
    option_vec,
    window::get_window_geometry
};

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::sync::{Arc, Mutex};

mod img;

// See this (to keep taking screenshots of the same window): https://stackoverflow.com/questions/5262413/does-xlib-have-an-active-window-event

lazy_static! {
    // A quantification of the differences between two screenshots (see crate::img::calc_diff()).
    // If the difference is higher or equal to this number, send the screenshot.
    // Min: 0 -> there is no difference at all
    // Max: DIFF_TOTAL -> the two images are completely different
    static ref IMG_DIFF: f32 = include_str!("../var/IMG_DIFF").to_owned().parse().unwrap();
}

static DIFF_TOTAL: f32 = 100.0;

// Delay between checking screenshots (in ms), the number must be equal or greater than the bot's requests delay
static SCREEN_DELAY: u64 = 4000;

// Listen for connections on this address
static LISTEN_ON: &str = "127.0.0.1:4040";

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
fn screenshooter(previous_scrn: &mut img::RgbImage, current_scrn: &mut img::RgbImage, must_send_prev_scrn: &mut bool, machine_kind: MachineKind, window: img::Window) {

    // Take a screenshot (and create a filename for it)
    let filename = Local::now().format("Screenshot_%H-%M-%S.png").to_string();
    let screenshot: img::RgbImage = img::screenshot_active_window(window, machine_kind, format!("{}{}", SAVE_PATH, filename)).expect("An error occurred during the screenshot process (filesystem I/O ?)");
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
        if diff >= *IMG_DIFF {
            if previous_scrn.height != 0 && previous_scrn.width != 0 {
                println!("    diff = {}, diff >= {} (IMG_DIFF)", diff, *IMG_DIFF);
                
                *must_send_prev_scrn = true;

            } else {
                *must_send_prev_scrn = false;
            }
        } else {
            println!("    diff = {}, diff < {} (IMG_DIFF)", diff, *IMG_DIFF);
            *must_send_prev_scrn = false;

            // If there are NO differences, also delete the image
            if diff == 0.0 {
                if let Err(_) = std::fs::remove_file(format!("screenshots/{}", filename)) {
                    println!("Warning: an error occurred while trying to delete a duplicated screenshot (diff = 0).")
                } else {
                    println!("Duplicated screenshot successfully deleted (diff = 0).")
                }
            }
        }
    } else {
        println!("The screenshot is empty (length = 0), not going to send it.");
        *must_send_prev_scrn = false;
    }
}

fn handle_client_conn(stream: &mut TcpStream, screenshot: img::RgbImage, must_send_scrn: bool, last_sent_scrn_hash: String) {

    // Screenshot's hash
    let scrn_hash = format!("{:x}", md5::compute(screenshot.as_vec_u8()));

    // If must_send_scrn is true and the screenshot hasn't been sent yet, send it 
    if must_send_scrn && last_sent_scrn_hash != scrn_hash {

        // Assert that the screenshot's data is not empty
        assert!(screenshot.data.len() != 0, "previous_screenshot's data doesn't exist.");

        let time_now = Local::now();
        println!("[{}] Sending image ({} x {})...", time_now.format("%H:%M:%S").to_string(), screenshot.width, screenshot.height);

        let img_bytes_num = screenshot.data.len() * 3;

        // Create the message to send (since TCP is a stream-based protocol, data is a stream - not a message)
        let mut msg: Vec<u8> = (screenshot.height.to_string()).as_bytes().to_owned(); // Start with the image height
        msg.push('|' as u8); // Insert a separator
        msg.append(&mut img_bytes_num.to_string().as_bytes().to_owned()); // Insert the image length (width * height * 3) in bytes
        msg.push('|' as u8); // Insert a separator
        msg.append(&mut screenshot.as_vec_u8().clone()); // Join it with the image data

        // Send image
        stream.write(&msg).expect("Uncaught stream error");
        println!("  Image sent!");
    } else {

        println!("No screenshot to send.");

        // Otherwise, send an empty message, meaning that the required screenshot has already been sent
        stream.write(&[]).expect("Uncaught stream error");
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

    let mut selected_window: img::Window = img::Window::new(0, 0, 0, 0, "0");
    
    match mkind {
        MachineKind::Unix => {
            println!("Unix-like machine detected.");

            println!("Select a window to screenshot by clicking the cursor on it...");
            
            // Get window id
            let select_cmd = Command::Window(SelectWindow);
            let window_id = String::from_utf8(
                xdotool::run(select_cmd, "").stdout
            ).unwrap();
            selected_window.id = window_id.clone();

            // Get window info (geometry)
            let window_geometry = get_window_geometry(&window_id, option_vec![]);
            let win_geom_output = String::from_utf8(window_geometry.stdout).unwrap();

            println!("{}", win_geom_output);
        
            // Parse the output of get_window_geometry(...)
            let regx = regex::Regex::new(r#"[0-9]+[x,][0-9]+"#).unwrap(); // Gets only position and geometry (size)

            // Add the captured values to selected_window
            {
                let captures = regx.captures(&win_geom_output).unwrap();

                println!("{:?}", captures);

                assert!(captures.len() == 2);

                // Split position into x and y (is printed as "x,y")
                let position: Vec<&str> = captures[0].split(',').collect();

                selected_window.x_pos = position[0].parse().unwrap();
                selected_window.y_pos = position[1].parse().unwrap();

            }

            println!("{:?}", selected_window); //DEBUG
        },
        MachineKind::Windows => {
            println!("Windows-like machine detected.");
        },
    }

    todo!();

    println!("The chosen value for IMG_DIFF is {}", *IMG_DIFF);

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
            loop {

                {
                    let mut previous_screenshot = previous_screenshot.lock().unwrap();
                    let mut current_screenshot = current_screenshot.lock().unwrap();
                    let mut must_send_prev_scrn = must_send_prev_scrn.lock().unwrap();
    
                    screenshooter(&mut *previous_screenshot, &mut *current_screenshot, &mut *must_send_prev_scrn, mkind, selected_window.clone());
                }
                
                // Separator
                println!("");
                
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

        std::thread::spawn ( move || {

            let previous_screenshot = previous_screenshot.lock().unwrap();
            let must_send_prev_scrn = must_send_prev_scrn.lock().unwrap();
            let last_sent_screenshot_hash = last_sent_screenshot_hash.lock().unwrap();

            // Remote address of client
            let mut stream = stream.expect("Stream error");
            let remote_addr = stream.peer_addr().unwrap();
            println!("Connected to {}", remote_addr.clone());

            let mut buffer: [u8;10] = [0;10];

            let read_bytes = stream.read(&mut buffer).expect("Error while reading data from TCP stream");            

            let mut msg = buffer[0..read_bytes].to_vec();

            while String::from_utf8(msg.to_vec()).unwrap() == String::from("more") {
                handle_client_conn(&mut stream, (*previous_screenshot).clone(), *must_send_prev_scrn, (*last_sent_screenshot_hash).clone());

                // Update buffer and message
                stream.read(&mut buffer).expect("Error while reading data from TCP stream");
                msg = buffer[0..read_bytes].to_vec();
            }

            println!("Disconnected from {}", remote_addr.clone());
        });
    }
}
