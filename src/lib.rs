use std::{borrow::Cow, collections::HashSet};

mod solver;
pub use solver::{Rank, Solver};

pub const MAX_GUESSES: usize = 32;

include!(concat!(env!("OUT_DIR"), "/dictionary.rs"));

pub struct Wordle {
    dictionary: HashSet<&'static str>,
}

impl Wordle {
    pub fn new() -> Self {
        Self {
            dictionary: HashSet::from_iter(DICTIONARY.into_iter().map(|(word, _)| word)),
        }
    }

    pub fn play<G: Guesser>(&self, answer: &'static str, mut guesser: G) -> Option<usize> {
        let mut history = Vec::new();

        for i in 1..=MAX_GUESSES {
            let guess = guesser.guess(&history);
            if guess == answer {
                return Some(i);
            }

            assert!(
                self.dictionary.contains(&*guess),
                "'{guess}' is not in the dictionary",
            );

            let mask = Correctness::compute(answer, &guess);
            history.push(Guess::new(guess, mask));
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Correctness {
    Correct,
    Misplaced,
    Wrong,
}

impl Correctness {
    fn compute(answer: &str, guess: &str) -> [Self; 5] {
        assert_eq!(answer.len(), 5);
        assert_eq!(guess.len(), 5);

        let mut mask = [Self::Wrong; 5];
        let mut misplaced = [0u8; (b'z' - b'a' + 1) as usize];

        for ((answer_byte, guess_byte), correctness) in
            answer.bytes().zip(guess.bytes()).zip(mask.iter_mut())
        {
            if answer_byte == guess_byte {
                *correctness = Correctness::Correct;
            } else {
                misplaced[(answer_byte - b'a') as usize] += 1;
            }
        }

        for (guess_byte, correctness) in guess.bytes().zip(mask.iter_mut()) {
            if *correctness == Correctness::Wrong && misplaced[(guess_byte - b'a') as usize] > 0 {
                *correctness = Correctness::Misplaced;
                misplaced[(guess_byte - b'a') as usize] -= 1;
            }
        }

        mask
    }
}

pub struct Guess<'a> {
    pub word: Cow<'a, str>,
    pub mask: [Correctness; 5],
}

impl Guess<'_> {
    pub fn new(word: String, mask: [Correctness; 5]) -> Self {
        Self {
            word: Cow::Owned(word),
            mask,
        }
    }

    fn allows(&self, word: &str) -> bool {
        Correctness::compute(word, &self.word) == self.mask
    }
}

pub trait Guesser {
    fn guess(&mut self, history: &[Guess]) -> String;
}

#[cfg(test)]
mod tests {
    macro_rules! mask {
        (C) => {$crate::Correctness::Correct};
        (M) => {$crate::Correctness::Misplaced};
        (W) => {$crate::Correctness::Wrong};
        ($($c:tt)+) => {[
            $(mask!($c)),+
        ]}
    }

    mod game {
        use crate::Wordle;

        macro_rules! guesser {
            (|$history:ident| $impl:block) => {{
                struct G;
                impl $crate::Guesser for G {
                    fn guess(&mut self, $history: &[$crate::Guess]) -> String {
                        $impl
                    }
                }
                G
            }};
        }

        #[test]
        fn genius() {
            let w = Wordle::new();
            let guesser = guesser!(|_history| { "right".to_string() });
            assert_eq!(w.play("right", guesser), Some(1));
        }

        #[test]
        fn magnificent() {
            let w = Wordle::new();
            let guesser = guesser!(|history| {
                if history.len() == 1 {
                    return "right".to_string();
                }
                return "wrong".to_string();
            });
            assert_eq!(w.play("right", guesser), Some(2));
        }

        #[test]
        fn impressive() {
            let w = Wordle::new();
            let guesser = guesser!(|history| {
                if history.len() == 2 {
                    return "right".to_string();
                }
                return "wrong".to_string();
            });
            assert_eq!(w.play("right", guesser), Some(3));
        }

        #[test]
        fn splendid() {
            let w = Wordle::new();
            let guesser = guesser!(|history| {
                if history.len() == 3 {
                    return "right".to_string();
                }
                return "wrong".to_string();
            });
            assert_eq!(w.play("right", guesser), Some(4));
        }

        #[test]
        fn great() {
            let w = Wordle::new();
            let guesser = guesser!(|history| {
                if history.len() == 4 {
                    return "right".to_string();
                }
                return "wrong".to_string();
            });
            assert_eq!(w.play("right", guesser), Some(5));
        }

        #[test]
        fn phew() {
            let w = Wordle::new();
            let guesser = guesser!(|history| {
                if history.len() == 5 {
                    return "right".to_string();
                }
                return "wrong".to_string();
            });
            assert_eq!(w.play("right", guesser), Some(6));
        }

        #[test]
        fn oops() {
            let w = Wordle::new();
            let guesser = guesser!(|_history| { "wrong".to_string() });
            assert_eq!(w.play("right", guesser), None);
        }
    }

    mod correctness {
        use crate::Correctness;

        #[test]
        fn all_green() {
            assert_eq!(Correctness::compute("abcde", "abcde"), mask![C C C C C]);
        }

        #[test]
        fn all_gray() {
            assert_eq!(Correctness::compute("abcde", "fghij"), mask![W W W W W]);
        }

        #[test]
        fn all_yellow() {
            assert_eq!(Correctness::compute("abcde", "eabcd"), mask![M M M M M]);
        }

        #[test]
        fn repeat_green() {
            assert_eq!(Correctness::compute("aabbb", "aaccc"), mask![C C W W W]);
        }

        #[test]
        fn repeat_yellow() {
            assert_eq!(Correctness::compute("aabbb", "ccaac"), mask![W W M M W]);
        }

        #[test]
        fn repeat_some_green() {
            assert_eq!(Correctness::compute("aabbb", "caacc"), mask![W C M W W]);
        }

        #[test]
        fn random_1() {
            assert_eq!(Correctness::compute("azzaz", "aaabb"), mask![C M W W W]);
        }

        #[test]
        fn random_2() {
            assert_eq!(Correctness::compute("baccc", "aaddd"), mask![W C W W W]);
        }

        #[test]
        fn random_3() {
            assert_eq!(Correctness::compute("abcde", "aacde"), mask![C W C C C]);
        }
    }
}
