extern crate screenshot_rs;
extern crate chrono;
extern crate image;
extern crate rayon;

use rayon::prelude::*;

use std::cmp::{max, min};
use std::sync::{Arc, Mutex};

use crate::MachineKind;

#[derive(Clone)]
pub struct RgbImage {
    pub data: Vec<[u8;3]>, // As a vector of pixels
    pub width: u32,
    pub height: u32
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
        assert!(width * height != data.len() as u32);

        RgbImage {
            data: data,
            width: width,
            height: height
        }
    }

    // Create a new instance from a vector of RGBA subpixels
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

// Calculate the average value of a vector of RGBA pixels using Rayon (parallelism)
fn parallel_avg(v: Vec<[u8;3]>) -> [u8;3] {
    // Average values for subpixels (excluding Alpha channel)
    let mut avg_r: u64 = 0;
    let mut avg_g: u64 = 0;
    let mut avg_b: u64 = 0;

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

    avg_r /= v.len() as u64;
    avg_g /= v.len() as u64;
    avg_b /= v.len() as u64;

    return [avg_r as u8, avg_g as u8, avg_b as u8];
}

// A quantification of the differences between two screenshots
// My own implementation
pub fn calc_diff(img1: RgbImage, img2: RgbImage) -> u16 {
    
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
    // Max: 100 -> the two images are completely different
    let difference_percentage: u16 = {
        (difference[0] as f32 / 256f32 * 100f32) as u16 +
        (difference[1] as f32 / 256f32 * 100f32) as u16 +
        (difference[2] as f32 / 256f32 * 100f32) as u16
    };

    difference_percentage
}

fn screenshot_active_window_unix(file: String) -> Result<RgbImage, ()> {
    // Screenshot the current active window and save it in path
    screenshot_rs::screenshot_window(file.clone());

    // Load the saved screenshot and return it
    let result = image::open(file);

    // Image as a vector of subpixels
    let img = match result {
        Ok(i) => i.to_rgb(),
        Err(_) => return Err(())
    };

    // Assert that they are actual RBGA subpixels and not something else, just in case
    assert!(img.len() % 3 == 0, "It's not a vector of RGBA subpixels");

    Ok(
        RgbImage::from_rgb(img.to_vec(), img.width(), img.height())
    )
}

// Use winapi (https://docs.rs/winapi/0.3.9/winapi/)
// See: https://web.archive.org/web/20161116203653/http://www.snippetsource.net/Snippet/158/capture-screenshot-in-c
fn screenshot_active_window_windows(file: String) -> Result<RgbImage, ()> {
    todo!()
}

// Public function to screenshot the currently active window. Save the screenshot
// in the file parameter, which should contain both the path and the filename
pub fn screenshot_active_window(mkind: MachineKind, file: String) -> Result<RgbImage, ()> {
    match mkind {
        MachineKind::Unix => screenshot_active_window_unix(file),
        MachineKind::Windows => screenshot_active_window_windows(file)
    }
}