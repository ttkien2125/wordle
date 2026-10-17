const ANSWERS: &str = include_str!("../answers.txt");

fn main() {
    let w = wordle::Wordle::new();
    for answer in ANSWERS.split_whitespace() {
        let guesser = wordle::algorithms::Naive::new();

        if let Some(score) = w.play(answer, guesser) {
            println!("found '{answer}' in {score} guesses");
            println!();
        } else {
            eprintln!("failed to guess");
        }
    }
}
