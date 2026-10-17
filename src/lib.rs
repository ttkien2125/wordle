use std::collections::HashSet;

pub mod algorithms;

pub const DICTIONARY: &str = include_str!("../dictionary.txt");
const MAX_GUESSES: usize = 32;

pub struct Wordle {
    dictionary: HashSet<&'static str>,
}

impl Wordle {
    pub fn new() -> Self {
        Self {
            dictionary: HashSet::from_iter(DICTIONARY.lines().map(|line| {
                line.split_once(' ')
                    .expect("every line is word + space + frequency")
                    .0
            })),
        }
    }

    pub fn play<G: Guesser>(&self, answer: &'static str, mut guesser: G) -> Option<usize> {
        let mut history = Vec::new();

        for i in 1..=MAX_GUESSES {
            let guess = guesser.guess(&history);
            if guess == answer {
                print!("history: ");
                for guess in history {
                    print!("{} ", guess.word);
                }
                println!();
                return Some(i);
            }

            assert!(
                self.dictionary.contains(&*guess),
                "'{guess}' is not in the dictionary",
            );

            let mask = Correctness::compute(answer, &guess);
            history.push(Guess { word: guess, mask });
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

        // Check for green
        for (index, (answer_char, guess_char)) in answer.chars().zip(guess.chars()).enumerate() {
            if answer_char == guess_char {
                mask[index] = Self::Correct;
            }
        }

        // Check for yellow
        let mut used = [false; 5];
        for (index, &correctness) in mask.iter().enumerate() {
            if correctness == Self::Correct {
                used[index] = true;
            }
        }

        for (index, guess_char) in guess.chars().enumerate() {
            if mask[index] == Self::Correct {
                continue;
            }

            if answer.chars().enumerate().any(|(index, answer_char)| {
                if answer_char == guess_char && !used[index] {
                    used[index] = true;
                    return true;
                }
                false
            }) {
                mask[index] = Self::Misplaced;
            }
        }

        mask
    }

    pub fn patterns() -> impl Iterator<Item = [Self; 5]> {
        itertools::iproduct!(
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong]
        )
        .map(From::from)
    }
}

pub struct Guess {
    pub word: String,
    pub mask: [Correctness; 5],
}

impl Guess {
    pub fn matches(&self, word: &str) -> bool {
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

    mod guess_matcher {
        macro_rules! check {
            ($prev:literal + [$($mask:tt)+] allows $next:literal) => {
                assert!(
                    $crate::Guess {
                        word: $prev.to_string(),
                        mask: mask![$($mask )+]
                    }
                    .matches($next)
                );
            };

            ($prev:literal + [$($mask:tt)+] disallows $next:literal) => {
                assert!(
                    !$crate::Guess {
                        word: $prev.to_string(),
                        mask: mask![$($mask )+]
                    }
                    .matches($next)
                );
            };
        }

        #[test]
        fn allows() {
            check!("abcde" + [C C C C C] allows "abcde");
            check!("abcde" + [W W W W W] allows "fghij");
            check!("abcde" + [M M M M M] allows "eabcd");
            check!("abcde" + [M M M M M] allows "eabcd");
            check!("baaaa" + [W C M W W] allows "aaccc");
        }

        #[test]
        fn disallows() {
            check!("abcdf" + [C C C C C] disallows "abcde");
            check!("baaaa" + [W C M W W] disallows "caacc");
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
