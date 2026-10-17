use std::{borrow::Cow, collections::HashMap, sync::OnceLock};

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static INITIAL: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();

struct Candidate {
    word: &'static str,
    score: f64,
}

pub struct PatternMap {
    candidates: Cow<'static, [(&'static str, usize)]>,
}

impl PatternMap {
    const FIRST_GUESS: &str = "tares";

    pub fn new() -> Self {
        Self {
            candidates: Cow::Borrowed(INITIAL.get_or_init(|| DICTIONARY.to_vec())),
        }
    }
}

impl Guesser for PatternMap {
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

        let mut pattern_weights = HashMap::with_capacity(243);

        for &(guess_word, _) in self.candidates.iter() {
            pattern_weights.clear();

            for &(target_word, frequency) in self.candidates.iter() {
                let mask = Correctness::compute(target_word, guess_word);
                *pattern_weights.entry(mask).or_insert(0) += frequency;
            }

            let expected_entropy = -pattern_weights
                .values()
                .map(|&weight| {
                    let p_pattern = weight as f64 / total_weight as f64;
                    p_pattern * p_pattern.log2()
                })
                .sum::<f64>();

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
