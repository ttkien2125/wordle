const ANSWERS: &str = include_str!("../answers.txt");

fn main() {
    let w = wordle::Wordle::new();
    for answer in ANSWERS.split_whitespace() {
        let guesser = wordle::algorithms::Naive::new();
        w.play(answer, guesser);
    }
}
