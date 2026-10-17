use clap::{Parser, ValueEnum};
use wordle::Guesser;

const ANSWERS: &str = include_str!("../answers.txt");

#[derive(Clone, ValueEnum)]
enum Algorithm {
    Naive,
}

#[derive(Parser)]
struct Args {
    #[arg(value_enum)]
    algorithm: Algorithm,

    #[arg(short, long)]
    max: Option<usize>,
}

fn main() {
    let args = Args::parse();

    match args.algorithm {
        Algorithm::Naive => play(wordle::algorithms::Naive::new, args.max),
    }
}

fn play<G: Guesser>(mut new: impl FnMut() -> G, max: Option<usize>) {
    let w = wordle::Wordle::new();
    for answer in ANSWERS.split_whitespace().take(max.unwrap_or(usize::MAX)) {
        let guesser = new();

        if let Some(score) = w.play(answer, guesser) {
            println!("found '{answer}' in {score} guesses");
            println!();
        } else {
            eprintln!("failed to guess");
        }
    }
}
