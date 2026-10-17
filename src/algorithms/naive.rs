use std::collections::HashMap;

use crate::{Correctness, DICTIONARY, Guess, Guesser};

struct Candidate {
    word: &'static str,
    score: f64,
}

pub struct Naive {
    candidates: HashMap<&'static str, usize>,
}

impl Naive {
    pub fn new() -> Self {
        Self {
            candidates: HashMap::from_iter(DICTIONARY.lines().map(|line| {
                let (word, frequency) = line
                    .split_once(' ')
                    .expect("every line is word + space + frequency");
                let frequency = frequency.parse().expect("frequency is a number");
                (word, frequency)
            })),
        }
    }
}

impl Guesser for Naive {
    fn guess(&mut self, history: &[Guess]) -> String {
        if history.is_empty() {
            return "tares".to_string();
        }

        if let Some(last_guess) = history.last() {
            self.candidates.retain(|word, _| last_guess.matches(word));
        }

        let total_weight: usize = self.candidates.values().sum();
        let mut best_candidate: Option<Candidate> = None;

        for guess_word in self.candidates.keys() {
            let mut expected_entropy = 0.0;

            for pattern in Correctness::patterns() {
                let hypothetical = Guess {
                    word: guess_word.to_string(),
                    mask: pattern,
                };

                let pattern_weight: usize = self
                    .candidates
                    .iter()
                    .filter_map(|(target_word, frequency)| {
                        if hypothetical.matches(target_word) {
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
