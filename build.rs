use std::io::prelude::Write;

const DICTIONARY: &str = include_str!("dictionary.txt");

fn main() {
    println!("cargo::rerun-if-changed=dictionary.txt");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut file = std::fs::File::create(out_dir.join("dictionary.rs"))
        .expect("could not create file in OUT_DIR");

    let mut words: Vec<_> = DICTIONARY
        .lines()
        .map(|line| {
            let (word, frequency) = line
                .split_once(' ')
                .expect("every line is word + space + frequency");
            let frequency: usize = frequency.parse().expect("frequency is a number");
            (word, frequency)
        })
        .collect();

    words.sort_unstable_by_key(|&(_, frequency)| std::cmp::Reverse(frequency));

    writeln!(
        file,
        "pub static DICTIONARY: [(&str, usize); {}] = [",
        words.len()
    )
    .unwrap();

    for (word, count) in words {
        writeln!(file, "(\"{word}\", {count}),").unwrap();
    }

    write!(file, "];").unwrap();
}
