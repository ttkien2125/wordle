use std::io::{Write, stdin, stdout};

use clap::{Parser, ValueEnum};
use wordle::{Guesser, Solver};

const ANSWERS: &str = include_str!("../answers.txt");
const MAX_BAR_WIDTH: usize = 40;

#[derive(Clone, ValueEnum)]
enum Rank {
    /// Pick based on E[information]
    ExpectedInformation,
    /// Pick based on p(word) + E[information]
    InfoPlusProbability,
    /// Pick based on p(word) * E[information]
    WeightedInformation,
    /// Pick based on E[score] = p(word) * (score + 1) + (1 - p(word)) * (score + E[guesses](entropy - E[information]))
    ExpectedScore,
}

#[derive(Parser)]
struct Args {
    /// Set how candidates are ranked at each step of the solver.
    ///
    /// By default, candidates will be ranked based on expected score.
    #[arg(value_enum, default_value = "expected-score")]
    rank_by: Rank,

    /// By default, counts will be smoothed using a sigmoid. This flag disables that.
    #[arg(long)]
    no_sigmoid: bool,

    /// By default, only the most likely 1/3 of candidates are considered at each step. This flag
    /// disables that pruning behavior.
    #[arg(long)]
    no_cut_off: bool,

    /// By default, statistics are printed at the end. This flag disables that.
    #[arg(long)]
    no_stats: bool,

    /// The number of games to run.
    ///
    /// If not passed, all Wordle games are run.
    #[arg(short, long, conflicts_with = "interactive")]
    games: Option<usize>,

    #[arg(short, long, conflicts_with = "games")]
    interactive: bool,
}

fn main() {
    let args = Args::parse();
    let mut solver = Solver::builder();

    solver.sigmoid = !args.no_sigmoid;
    solver.cut_off = !args.no_cut_off;

    solver.rank_by = match args.rank_by {
        Rank::ExpectedInformation => wordle::Rank::ExpectedInformation,
        Rank::InfoPlusProbability => wordle::Rank::InfoPlusProbability,
        Rank::WeightedInformation => wordle::Rank::WeightedInformation,
        Rank::ExpectedScore => wordle::Rank::ExpectedScore,
    };

    if args.interactive {
        play_interactive(|| solver.build());
    } else {
        play(|| solver.build(), &args);
    }
}

fn play_interactive<G: Guesser>(new: impl FnOnce() -> G) {
    let mut guesser = new();
    let mut history = Vec::new();
    let mut input = String::with_capacity(8);

    println!("C: Correct / Green, M: Misplaced / Yellow, W: Wrong / Gray");

    for turn in 1..=6 {
        let guess = guesser.guess(&history);
        println!("Guess {turn}: {}", guess.to_uppercase());

        let mask = loop {
            print!("Colors:  ");

            stdout().flush().expect("failed to flush stdout");

            input.clear();
            stdin().read_line(&mut input).expect("failed to read input");

            match parse_mask(&input) {
                Ok(mask) => break mask,
                Err(err) => {
                    eprintln!("   Invalid input - {err}");
                    eprintln!("   Use 5 chars of C/M/W (or G/Y/X)");
                }
            }
        };

        if mask == [wordle::Correctness::Correct; 5] {
            println!("Solved in {turn} guesses!");
            return;
        }

        history.push(wordle::Guess::new(guess, mask));
    }
}

fn play<G: Guesser>(mut new: impl FnMut() -> G, args: &Args) {
    let w = wordle::Wordle::new();

    let mut score_counts = [0; wordle::MAX_GUESSES];

    for answer in ANSWERS
        .split_whitespace()
        .take(args.games.unwrap_or(usize::MAX))
    {
        let guesser = new();

        if let Some(score) = w.play(answer, guesser) {
            score_counts[score] += 1;
            println!("found '{answer}' in {score} guesses");
        } else {
            eprintln!("failed to guess '{answer}'");
        }
    }

    if !args.no_stats {
        print_statistics(score_counts);
    }
}

fn parse_mask(input: &str) -> Result<[wordle::Correctness; 5], String> {
    input
        .trim()
        .chars()
        .filter(|correctness| !correctness.is_whitespace())
        .map(|correctness| match correctness.to_ascii_uppercase() {
            'C' => Ok(wordle::Correctness::Correct),
            'M' => Ok(wordle::Correctness::Misplaced),
            'W' => Ok(wordle::Correctness::Wrong),
            _ => Err(format!("Invalid character '{correctness}'")),
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|v: Vec<_>| format!("Expected 5 letters, got {}", v.len()))
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
