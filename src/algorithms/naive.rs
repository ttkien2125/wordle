use std::borrow::Cow;

use crate::{Correctness, DICTIONARY, Guess, Guesser};

struct Candidate {
    word: &'static str,
    score: f64,
}

pub struct Naive {
    candidates: Vec<(&'static str, usize)>,
}

impl Naive {
    const FIRST_GUESS: &str = "tares";

    pub fn new() -> Self {
        Self {
            candidates: DICTIONARY.to_vec(),
        }
    }
}

impl Guesser for Naive {
    fn guess(&mut self, history: &[Guess]) -> String {
        if history.is_empty() {
            return Self::FIRST_GUESS.to_string();
        }

        if let Some(last_guess) = history.last() {
            self.candidates.retain(|(word, _)| last_guess.allows(word));
        }

        let total_weight: usize = self.candidates.iter().map(|(_, frequency)| frequency).sum();
        let mut best_candidate: Option<Candidate> = None;

        for (guess_word, _) in &self.candidates {
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
                .is_none_or(|best| expected_entropy > best.score);

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
