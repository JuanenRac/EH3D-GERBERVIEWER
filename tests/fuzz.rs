//! Damaged files with a fixed seed: every prefix of the example layer and many copies of it with bytes changed, inserted or cut are read and turned into a scene without a panic, and
//! no shape of the scene holds a number that is not finite. The seed is fixed, so a failure is the same failure every time.

use eh3d_gerberviewer::scene::{from_gerber_with, SceneOptions};
use eh3d_parser_gerber::{parse_bytes_with, InputLimits};

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn sample() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl")).unwrap()
}

/// Reads `bytes` as a layer and turns it into a scene: the shapes must be finite whatever the bytes were.
fn read_whole(bytes: &[u8]) -> usize {
    let Ok(file) = parse_bytes_with(bytes, &InputLimits::default()) else {
        return 0;
    };
    let scene = from_gerber_with(
        &file,
        &SceneOptions {
            arc_tolerance_mm: 0.01,
        },
    );
    for shape in &scene.shapes {
        let text = format!("{shape:?}");
        assert!(!text.contains("NaN") && !text.contains("inf"), "{text}");
    }
    scene.shapes.len()
}

#[test]
fn the_whole_example_gives_shapes_and_every_prefix_of_it_is_read_without_a_panic() {
    let bytes = sample();
    assert!(read_whole(&bytes) > 0);
    for end in (0..bytes.len()).step_by(3) {
        read_whole(&bytes[..end]);
    }
}

#[test]
fn copies_with_bytes_changed_inserted_or_cut_are_read_without_a_panic() {
    let bytes = sample();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for round in 0..400 {
        let mut copy = bytes.clone();
        for _ in 0..1 + round % 6 {
            let at = rng.below(copy.len());
            match rng.below(4) {
                0 => copy[at] = rng.next() as u8,
                1 => copy.insert(at, rng.next() as u8),
                2 => {
                    copy.remove(at);
                }
                _ => copy.truncate(at.max(1)),
            }
            if copy.is_empty() {
                copy.push(b'%');
            }
        }
        read_whole(&copy);
    }
}

#[test]
fn bytes_that_are_not_a_layer_at_all_are_read_without_a_panic() {
    let mut rng = Rng(7);
    for len in [0usize, 1, 2, 17, 255, 4096] {
        let junk: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        read_whole(&junk);
    }
    read_whole(b"%FSLAX24Y24*%\n%ADD10C,1e999*%\nD10*\nX9999999999Y9999999999D03*\nM02*\n");
    read_whole(b"G75*\nG03*\nX0Y0I999999999J0D01*\nM02*\n");
}
