use std::borrow::Cow;

use once_cell::sync::OnceCell;

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static INITIAL: OnceCell<Vec<(&'static str, usize)>> = OnceCell::new();

struct Candidate {
    word: &'static str,
    score: f64,
}

pub struct OnceInit {
    candidates: Cow<'static, Vec<(&'static str, usize)>>,
}

impl OnceInit {
    const FIRST_GUESS: &str = "tares";

    pub fn new() -> Self {
        Self {
            candidates: Cow::Borrowed(INITIAL.get_or_init(|| {
                Vec::from_iter(DICTIONARY.lines().map(|line| {
                    let (word, frequency) = line
                        .split_once(' ')
                        .expect("every line is word + space + frequency");
                    let frequency = frequency.parse().expect("frequency is a number");
                    (word, frequency)
                }))
            })),
        }
    }
}

impl Guesser for OnceInit {
    fn guess(&mut self, history: &[Guess]) -> String {
        if history.is_empty() {
            return Self::FIRST_GUESS.to_string();
        }

        if let Some(last_guess) = history.last() {
            if let Cow::Owned(ref mut candidates) = self.candidates {
                candidates.retain(|(word, _)| last_guess.allows(word));
            } else {
                self.candidates = Cow::Owned(
                    self.candidates
                        .iter()
                        .filter(|(word, _)| last_guess.allows(word))
                        .copied()
                        .collect(),
                );
            }
        }

        let total_weight: usize = self.candidates.iter().map(|(_, frequency)| frequency).sum();
        let mut best_candidate: Option<Candidate> = None;

        for (guess_word, _) in self.candidates.iter() {
            let mut expected_entropy = 0.0;

            for pattern in Correctness::patterns() {
                let hypothetical = Guess {
                    word: Cow::Borrowed(guess_word),
                    mask: pattern,
                };

                let pattern_weight: usize = self
                    .candidates
                    .iter()
                    .filter_map(|(target_word, frequency)| {
                        if hypothetical.allows(target_word) {
                            return Some(frequency);
                        }
                        None
                    })
                    .sum();

                if pattern_weight == 0 {
                    continue;
                }

                let p_pattern = pattern_weight as f64 / total_weight as f64;
                expected_entropy -= p_pattern * p_pattern.log2();
            }

            let is_better = best_candidate
                .as_ref()
                .map_or(true, |best| expected_entropy > best.score);

            if is_better {
                best_candidate = Some(Candidate {
                    word: guess_word,
                    score: expected_entropy,
                });
            }
        }

        best_candidate.unwrap().word.to_string()
    }
}
