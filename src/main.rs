use std::collections::BTreeMap;

use clap::{Parser, ValueEnum};
use wordle::Guesser;

const ANSWERS: &str = include_str!("../answers.txt");

#[derive(Clone, ValueEnum)]
enum Algorithm {
    Naive,
    OnceInit,
    PatternMap,
    ArrayEntropy,
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
        Algorithm::OnceInit => play(wordle::algorithms::OnceInit::new, args.max),
        Algorithm::PatternMap => play(wordle::algorithms::PatternMap::new, args.max),
        Algorithm::ArrayEntropy => play(wordle::algorithms::ArrayEntropy::new, args.max),
    }
}

fn play<G: Guesser>(mut new: impl FnMut() -> G, max: Option<usize>) {
    let w = wordle::Wordle::new();

    let mut answers_guessed = 0;
    let mut games_won = 0;
    let mut score_map = BTreeMap::new();
    let mut total_score = 0;
    let mut min_score = usize::MAX;
    let mut max_score = usize::MIN;

    for answer in ANSWERS.split_whitespace().take(max.unwrap_or(usize::MAX)) {
        let guesser = new();

        if let Some(score) = w.play(answer, guesser) {
            answers_guessed += 1;

            if score <= 6 {
                games_won += 1;
            }

            *score_map.entry(score).or_insert(0) += 1;
            total_score += score;
            min_score = min_score.min(score);
            max_score = max_score.max(score);

            println!("found '{answer}' in {score} guesses");
            println!();
        } else {
            eprintln!("failed to guess");
        }
    }

    let average_score = total_score as f64 / answers_guessed as f64;
    let win_percentage = 100.0 * games_won as f64 / answers_guessed as f64;

    println!("average score: {average_score:.2}");
    println!("min score: {min_score} - max score: {max_score}");
    println!("score distribution:");
    for (score, num_games) in score_map {
        println!(" - score {score}: {num_games} games");
    }
    println!("games won: {games_won} out of {answers_guessed}");
    println!("win percentage: {win_percentage:.2}%");
}
