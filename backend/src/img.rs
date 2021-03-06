extern crate screenshot_rs;
extern crate chrono;
extern crate rayon;
extern crate png;

use rayon::prelude::*;
use png::{Decoder, Limits};

use std::fs::File;
use std::cmp::{max, min};
use std::sync::{Arc, Mutex};

use crate::MachineKind;
use crate::DIFF_TOTAL;

#[derive(Clone, Debug)]
pub struct RgbImage {
    pub data: Vec<[u8;3]>, // As a vector of pixels
    pub width: u32,
    pub height: u32
}

#[derive(Clone, Debug)]
pub struct Window {
    pub x_pos: u32,
    pub y_pos: u32,
    pub width: u32,
    pub height: u32,
    pub id: String // Could be an integer, but just to be sure
}

impl RgbImage {

    // Empty image
    pub fn new() -> RgbImage {
        RgbImage {
            data: vec![],
            width: 0,
            height: 0
        }
    }

    pub fn from_pixels(data: Vec<[u8;3]>, height: u32, width: u32) -> RgbImage {

        // Assert that data is correct
        assert!(width * height == data.len() as u32);

        RgbImage {
            data: data,
            width: width,
            height: height
        }
    }

    // Create a new instance from a vector of RGB subpixels
    pub fn from_rgb(data: Vec<u8>, height: u32, width: u32) -> RgbImage {
        let mut correct_data: Vec<[u8;3]> = vec![];

        // Assert that there are all the subpixels
        assert!(data.len() % 3 == 0);

        let mut buffer: [u8;3] = [0, 0, 0];
        let mut buffer_index: usize = 0;

        for subpixel in data {

            buffer[buffer_index] = subpixel;

            if buffer_index == buffer.len() - 1 {
                correct_data.push(buffer.clone());
                buffer_index = 0;
            } else {
                buffer_index += 1;
            }
        }

        RgbImage::from_pixels(correct_data, width, height)
    }

    pub fn as_vec_u8(&self) -> Vec<u8> {
        let mut output: Vec<u8> = vec![];
        
        for pixel in self.data.clone() {
            for subp in pixel.iter() {
                output.push(subp.clone());
            }
        }

        output
    }
}

impl Window {
    pub fn new<T: ToString>(xpos: u32, ypos: u32, width: u32, height: u32, id: T) -> Window {
        Window {
            x_pos: xpos,
            y_pos: ypos,
            width: width,
            height: height,
            id: id.to_string()
        }
    }

    pub fn take_screenshot(&self, save_path: String) -> RgbImage {
        // Take a screenshot and save it
        screenshot_rs::screenshot_area(save_path.clone(), false);

        // Load the saved screenshot and return it
        //let result = image::open(file);
        let decoder = Decoder::new_with_limits(File::open(save_path).unwrap(), Limits::default());
        let mut reader = decoder.read_info().unwrap().1;

        let mut img_data: Vec<u8> = vec![];

        // Fill img_data by reading data from reader
        let mut row_buffer: Option<&[u8]> = reader.next_row().unwrap();
        while let Some(buffer) = row_buffer {
            //vec![img_data, buffer.to_vec()].concat();
            img_data.extend_from_slice(buffer);
            row_buffer = reader.next_row().unwrap();
        }

        // Assert that they are actual RGB subpixels and not something else, just in case
        assert!(img_data.len() % 3 == 0, "It's not a vector of RGB subpixels");

        RgbImage::from_rgb(img_data, self.width, self.height)

    }
}

// Calculate the average value of a vector of RGB pixels using Rayon (parallelism)
fn parallel_avg(v: Vec<[u8;3]>) -> [u8;3] {

    // Average values for subpixels (excluding Alpha channel)
    let mut avg_r: u64 = 0;
    let mut avg_g: u64 = 0;
    let mut avg_b: u64 = 0;

    assert!(v.len() != 0, "Cannot divide by zero");

    // Keep memorized the vector length
    let v_len = v.len();

    {
        // Copy the values to do the calculations
        let avg_r_arcmutex: Arc<Mutex<u64>> = Arc::new(Mutex::new(avg_r));
        let avg_g_arcmutex: Arc<Mutex<u64>> = Arc::new(Mutex::new(avg_g));
        let avg_b_arcmutex: Arc<Mutex<u64>> = Arc::new(Mutex::new(avg_b));
        
        // Clone the values to effectively use Arc-Mutex
        let avg_r_clone = avg_r_arcmutex.clone();
        let avg_g_clone = avg_g_arcmutex.clone();
        let avg_b_clone = avg_b_arcmutex.clone();
    
        // Sum the values of red, green and blue (separately) in parallel
        v.par_iter()
            .for_each( |&pixel| {
    
                let mut avg_r_clone = avg_r_clone.lock().unwrap();
                let mut avg_g_clone = avg_g_clone.lock().unwrap();
                let mut avg_b_clone = avg_b_clone.lock().unwrap();
    
                *avg_r_clone += pixel[0] as u64;
                *avg_g_clone += pixel[1] as u64;
                *avg_b_clone += pixel[2] as u64;
            });
            
        // Copy the final values in the original variables
        avg_r = *(avg_r_arcmutex.lock().unwrap());
        avg_g = *(avg_g_arcmutex.lock().unwrap());
        avg_b = *(avg_b_arcmutex.lock().unwrap());
    }

    avg_r /= v_len as u64;
    avg_g /= v_len as u64;
    avg_b /= v_len as u64;

    return [avg_r as u8, avg_g as u8, avg_b as u8];
}

// A quantification of the differences between two screenshots
// My own implementation
pub fn calc_diff(img1: RgbImage, img2: RgbImage) -> f32 {

    // If sizes are different return DIFF_TOTAL
    //if img1.data.len() != img2.data.len() {
    //    return DIFF_TOTAL;
    //}
    
    // Calculate the average color for each image
    let avg_color_img1 = parallel_avg(img1.data);
    let avg_color_img2 = parallel_avg(img2.data);

    // Calculate the difference of the two colors (the highest minus the lowest)
    let difference: [u8;3] = [
        max(avg_color_img1[0], avg_color_img2[0]) - min(avg_color_img1[0], avg_color_img2[0]),
        max(avg_color_img1[1], avg_color_img2[1]) - min(avg_color_img1[1], avg_color_img2[1]),
        max(avg_color_img1[2], avg_color_img2[2]) - min(avg_color_img1[2], avg_color_img2[2])
    ];

    // Min: 0 -> there is no difference at all
    // Max: DIFF_TOTAL -> the two images are completely different
    let difference_percentage: f32 = {
        (difference[0] as f32 / 255.0f32 * DIFF_TOTAL) +
        (difference[1] as f32 / 255.0f32 * DIFF_TOTAL) +
        (difference[2] as f32 / 255.0f32 * DIFF_TOTAL)
    };

    difference_percentage
}

fn screenshot_active_window_unix(window: Window, file: String) -> Result<RgbImage, ()> {
    
    Ok(window.take_screenshot(file))
}

// Use winapi (https://docs.rs/winapi/0.3.9/winapi/)
// See: https://web.archive.org/web/20161116203653/http://www.snippetsource.net/Snippet/158/capture-screenshot-in-c
fn screenshot_active_window_windows(file: String) -> Result<RgbImage, ()> {
    todo!()
}

// Public function to screenshot the currently active window. Save the screenshot
// in the file parameter, which should contain both the path and the filename
pub fn screenshot_active_window(window: Window, mkind: MachineKind, file: String) -> Result<RgbImage, ()> {
    match mkind {
        MachineKind::Unix => screenshot_active_window_unix(window, file),
        MachineKind::Windows => screenshot_active_window_windows(file)
    }
}