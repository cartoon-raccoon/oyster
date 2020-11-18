use linefeed::complete::{Completion, Completer};
use linefeed::terminal::Terminal;
use linefeed::prompter::Prompter;

// First implementation of the completer.
// It does nothing as of yet, but it should work soon.
pub struct OshComplete {

}

impl<Term: Terminal> Completer<Term> for OshComplete {
    fn complete(
        &self,
        word: &str,
        _prompter: &Prompter<Term>,
        start: usize,
        end: usize,
    ) -> Option<Vec<Completion>> {
        println!("\n{} {} {}", word, start, end);
        Some(Vec::new())
    }
}