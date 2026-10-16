use crate::Guesser;

pub struct Naive;

impl Naive {
    pub fn new() -> Self {
        Self
    }
}

impl Guesser for Naive {
    fn guess(&mut self, _history: &[crate::Guess]) -> String {
        todo!()
    }
}
