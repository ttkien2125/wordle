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

    let mut score_counts = [0; wordle::MAX_GUESSES];

    for answer in ANSWERS.split_whitespace().take(max.unwrap_or(usize::MAX)) {
        let guesser = new();

        if let Some(score) = w.play(answer, guesser) {
            score_counts[score] += 1;
            println!("found '{answer}' in {score} guesses");
        } else {
            eprintln!("failed to guess '{answer}'");
        }
    }

    print_statistics(score_counts);
}

fn print_statistics(score_counts: [usize; wordle::MAX_GUESSES]) {
    let answers_guessed: usize = score_counts.iter().sum();
    let games_won: usize = score_counts[1..=6].iter().sum();
    let min_score = score_counts
        .iter()
        .position(|&count| count > 0)
        .unwrap_or(0);
    let max_score = score_counts
        .iter()
        .rposition(|&count| count > 0)
        .unwrap_or(0);
    let total_score: usize = score_counts
        .iter()
        .enumerate()
        .map(|(score, &count)| score * count)
        .sum();

    let average_score = total_score as f64 / answers_guessed as f64;
    let win_percentage = 100.0 * games_won as f64 / answers_guessed as f64;

    println!("\n========== statistics ==========");
    println!("average score: {average_score:.6}");
    println!("min score: {min_score} - max score: {max_score}");
    println!("games won: {games_won} out of {answers_guessed} ({win_percentage:.2}%)");

    let max_count = score_counts.iter().copied().max().unwrap_or(0);
    const MAX_BAR_WIDTH: usize = 40;

    println!("score distribution:");
    for (score, &count) in score_counts.iter().take(max_score + 1).enumerate().skip(1) {
        let bar_length = (count * MAX_BAR_WIDTH).checked_div(max_count).unwrap_or(0);
        let bar = "█".repeat(bar_length);
        let percentage = if answers_guessed > 0 {
            100.0 * count as f64 / answers_guessed as f64
        } else {
            0.0
        };
        let marker = if score <= 6 { " " } else { "!" };
        println!("{marker}{score:2} | {bar:<MAX_BAR_WIDTH$} {count:>4} ({percentage:>5.1}%)");
    }
}
