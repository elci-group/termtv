//! Integration test for the native TTV1 format.

use std::fs::File;
use std::io::Write;

use termtv::{Cell, Rgb, Video};

#[test]
fn ttv1_round_trip() {
    let tmp = std::env::temp_dir().join("termtv_round_trip.ttv");
    let _ = std::fs::remove_file(&tmp);

    let width = 4;
    let height = 2;
    let cells = width * height;
    let frame: Vec<Cell> = (0..cells)
        .map(|i| Cell {
            top: Rgb(i as u8, (i * 2) as u8, (i * 3) as u8),
            bottom: Rgb((i * 4) as u8, (i * 5) as u8, (i * 6) as u8),
        })
        .collect();

    {
        let mut file = File::create(&tmp).unwrap();
        file.write_all(b"TTV1").unwrap();
        file.write_all(&(width as u16).to_le_bytes()).unwrap();
        file.write_all(&(height as u16).to_le_bytes()).unwrap();
        file.write_all(&(30_000_u32).to_le_bytes()).unwrap();
        file.write_all(&2_u64.to_le_bytes()).unwrap();
        for _ in 0..2 {
            for cell in &frame {
                file.write_all(&[cell.top.0, cell.top.1, cell.top.2])
                    .unwrap();
                file.write_all(&[cell.bottom.0, cell.bottom.1, cell.bottom.2])
                    .unwrap();
            }
        }
    }

    let video = Video::open(&tmp).unwrap();
    assert_eq!(video.width, width);
    assert_eq!(video.height, height);
    assert_eq!(video.fps, 30.0);
    assert_eq!(video.frames.len(), 2);
    assert_eq!(video.frames[0], frame);
    assert_eq!(video.frames[1], frame);

    let _ = std::fs::remove_file(&tmp);
}
