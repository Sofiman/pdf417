#![allow(dead_code)]

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";
const PADDING: usize = 4;

pub fn display_bits(width: usize, bits: &[bool]) {
    let quiet_zone_v = str::repeat(WHITE, width + PADDING * 2);
    let quiet_zone_h = &quiet_zone_v[..PADDING * WHITE.len()];

    println!("{quiet_zone_v}\n{quiet_zone_v}");
    for chunk in bits.chunks(width) {
        print!("{quiet_zone_h}");
        for &on in chunk { print!("{}", if on { BLACK } else { WHITE }); }
        println!("{quiet_zone_h}");
    }
    println!("{quiet_zone_v}\n{quiet_zone_v}\x1B[0m");
}

pub fn display_bitmap(width: usize, bitmap: &[u8]) {
    let quiet_zone_v = str::repeat(WHITE, width + PADDING * 2);
    let quiet_zone_h = &quiet_zone_v[..PADDING * WHITE.len()];

    println!("{quiet_zone_v}\n{quiet_zone_v}");

    let mut col = 0;
    for b in bitmap {
        if col == 0 {
            print!("{quiet_zone_h}");
        }

        for i in (0..8).rev() {
            print!("{}", if (b >> i) & 1 != 0 { BLACK } else { WHITE });

            col += 1;
            if col == width {
                println!("{quiet_zone_h}");
                col = 0;
                break;
            }
        }
    }
    println!("{quiet_zone_v}\n{quiet_zone_v}\x1B[0m");
}
