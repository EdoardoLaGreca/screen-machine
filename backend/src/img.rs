extern crate screenshot_rs;
extern crate chrono;
extern crate image;
extern crate rayon;

use rayon::prelude::*;

use crate::MachineKind;

pub struct RgbaImage {
    pub data: Vec<[u8;4]>, // As a vector of pixels
    pub width: u32,
    pub height: u32
}

impl RgbaImage {

    pub fn new(data: Vec<[u8;4]>, height: u32, width: u32) -> RgbaImage {

        // Assert that data is correct
        assert!(width * height != data.len() as u32);

        RgbaImage {
            data: data,
            width: width,
            height: height
        }
    }

    // Create a new instance from a vector of RGBA subpixels
    pub fn from_rgba(data: Vec<u8>, height: u32, width: u32) -> RgbaImage {
        let correct_data: Vec<[u8;4]> = vec![];

        // Assert that there are all the subpixels
        assert!(data.len() % 4 == 0);

        let mut buffer: [u8;4] = [0, 0, 0, 0];
        let mut buffer_index: usize = 0;

        for subpixel in data {

            buffer[buffer_index] = subpixel;

            if buffer_index == buffer.len() - 1 {
                correct_data.push(buffer.clone());
                buffer_index = 0;
            }
        }

        RgbaImage::new(correct_data, width, height)
    }

    pub fn as_vec_u8(&self) -> Vec<u8> {
        let output: Vec<u8> = vec![];
        
        for pixel in self.data {
            for subp in pixel.iter() {
                output.push(subp.clone());
            }
        }

        output
    }
}

/* -- REWRITE --
// Calculate the average value of a vector of RGBA pixels using Rayon (parallelism)
fn parallel_avg(v: Vec<[u8;4]>) -> [u64;3] {

    // Average values for subpixels (excluding Alpha channel)
    let mut avg_r: u64 = 0;
    let mut avg_g: u64 = 0;
    let mut avg_b: u64 = 0;

    v.par_iter()
        .map( |&pixel| {
            avg_r += pixel[0] as u64;
            avg_g += pixel[1] as u64;
            avg_b += pixel[2] as u64;
        });
    
    avg_r /= v.len() as u64;
    avg_g /= v.len() as u64;
    avg_b /= v.len() as u64;

    return [avg_r, avg_g, avg_b];
}

// My own implementation
pub fn calc_diff(img1: RgbaImage, img2: RgbaImage) -> u16 {
    // Calculate the average color of the two images
    let avg1 = parallel_avg(img1.data);
    let avg2 = parallel_avg(img2.data);

    // Put the numbers in a scale from 0 to 1000 (255 is the max value)
    let scaled_avg1 = avg1 / 255 * 1000;
    let scaled_avg2 = avg2 / 255 * 1000;

    // And subtract one from the other

}
*/

fn screenshot_active_window_unix(file: String) -> Result<RgbaImage, ()> {
    // Screenshot the current active window and save it in path
    screenshot_rs::screenshot_window(file);

    // Load the saved screenshot and return it
    let result = image::open(file);

    // Image as a vector of subpixels
    let img = match result {
        Ok(i) => i.to_rgba(),
        Err(_) => return Err(())
    };

    // Assert that they are actual RBGA subpixels and not something else, just in case
    assert!(img.len() % 4 == 0, "It's not a vector of RGBA subpixels");

    let img_as_pixels: Vec<[u8; 4]> = vec![];

    Ok(
        RgbaImage::from_rgba(img.to_vec(), img.width(), img.height())
    )
}


fn screenshot_active_window_windows(file: String) -> Result<RgbaImage, ()> {
    todo!()
}

// Public function to screenshot the currently active window. Save the screenshot
// in the file parameter, which should contain both the path and the filename
pub fn screenshot_active_window(mkind: MachineKind, file: String) -> Result<RgbaImage, ()> {
    match mkind {
        MachineKind::Unix => screenshot_active_window_unix(file),
        MachineKind::Windows => screenshot_active_window_windows(file)
    }
}