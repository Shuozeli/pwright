use pwright_file_input_fs::{MimeLookup, PinnedMime};

fn main() {
    for (index, name) in std::env::args().skip(1).enumerate() {
        assert!(index < 128 && name.len() <= 4096, "diagnostic input limit");
        println!("{}", PinnedMime.lookup(&name).unwrap_or("-"));
    }
}
