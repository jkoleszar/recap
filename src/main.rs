use std::borrow::Cow::{self, Borrowed, Owned};
use std::mem::MaybeUninit;

use anyhow::Result;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use nom::error::ErrorKind;
use rustyline::completion::FilenameCompleter;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};
use rustyline::{Completer, Helper, Hinter, Validator};

use recap::{human, vm};

type Span<'a> = nom_locate::LocatedSpan<&'a str>;
struct ParseError(Diagnostic<()>);
struct File(SimpleFile<&'static str, String>);

impl<'a> nom::error::ParseError<Span<'a>> for ParseError {
    fn from_error_kind(input: Span<'a>, kind: ErrorKind) -> Self {
        let start = input.location_offset();
        let end = start + input.fragment().len();
        let message = format!("expected {}", kind.description());
        Self(
            Diagnostic::error()
                .with_message("parse error")
                .with_labels(vec![Label::primary((), start..end).with_message(message)]),
        )
    }
    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        let start = input.location_offset();
        let end = start + input.fragment().len();
        let message = format!("while expecting {}", kind.description());
        Self(other.with_labels(vec![Label::secondary((), start..end).with_message(message)]))
    }
}

#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

// To debug rustyline:
// RUST_LOG=rustyline=debug cargo run --example example 2> debug.log
fn main() -> Result<()> {
    env_logger::init();

    // Configure rustyline
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();
    let h = MyHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    let mut count = 1;

    // Configure codespan-reporting
    let csr_writer = StandardStream::stdout(ColorChoice::Always);
    let csr_config = codespan_reporting::term::Config::default();

    // Configure recap
    let mut memory = [MaybeUninit::uninit(); 100];
    let _ = vm::Machine::new(&mut memory);

    loop {
        let p = format!("{count}> ");
        rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{p}\x1b[0m");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                use nom::error::VerboseError;
                rl.add_history_entry(line.as_str())?;
                let file = SimpleFile::new("stdin", line);
                for token in human::tokenize::<ParseError>(Span::new(file.source())) {
                    match token {
                        Ok(span) => println!("{span:?}"),
                        Err(e) => codespan_reporting::term::emit(
                            &mut csr_writer.lock(),
                            &csr_config,
                            &file,
                            &e.0,
                        )?,
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("Encountered Eof");
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
        count += 1;
    }
    rl.append_history("history.txt")?;
    Ok(())
}
