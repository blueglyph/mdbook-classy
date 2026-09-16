mod tests;

use clap::{arg, command, ArgMatches, Command};
use mdbook_core::book::{Book, Chapter};
use mdbook_core::errors::{Error, Result};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use mdbook_markdown::{new_cmark_parser, MarkdownOptions};
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::io;
use std::io::Read;
use std::process;

#[derive(Default)]
pub struct Classy;

impl Classy {
    pub fn new() -> Classy {
        Classy
    }
}

impl Preprocessor for Classy {
    fn name(&self) -> &str {
        "classy"
    }
    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        book.for_each_mut(|book_item| {
            if let mdbook_core::book::BookItem::Chapter(chapter) = book_item {
                if let Err(e) = classy(chapter) {
                    eprintln!("classy error: {:?}", e);
                }
            }
        });
        Ok(book)
    }
    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        Ok(renderer == "html")
    }
}

#[derive(Debug, Clone, Copy)]
enum State<'a> {
    BeforeStart,
    Expecting,
    Accepted(&'a str),
    Inner,
}

/// This is where the markdown transformation actually happens.
/// Take paragraphs beginning with `{:.class-name}` and give them special rendering.
/// Mutation: the payload here is that it edits chapter.content.
fn classy(chapter: &mut Chapter) -> Result<(), Error> {
    const VERBOSE: bool = false;
    let parser = new_cmark_parser(&chapter.content, &MarkdownOptions::default());

    let mut state = State::BeforeStart;
    let mut new_events = vec![];

    for event in parser {
        if VERBOSE { eprintln!("## event {event:?}, state {state:?}"); }
        if let State::Accepted(text) = state {
            if matches!(event, Event::SoftBreak | Event::HardBreak) {
                state = State::Inner;
                new_events.push(Event::Start(Tag::Paragraph));
            } else {
                state = State::BeforeStart;
                new_events.pop().unwrap();
                new_events.push(Event::Start(Tag::Paragraph));
                new_events.push(Event::Text(text.into()));
                new_events.push(event);
            }
            continue;
        }
        match event {
            Event::Start(Tag::Paragraph) => {
                state = State::Expecting;
            }

            Event::Text(CowStr::Borrowed(text)) if matches!(state, State::Expecting) => {
                // "{:.class}", "{:.class1 class2}", ...
                // event sequence: Start(Paragraph) -> Text(Borrowed(_)) -> SoftBreak | HardBreak
                if text.len() > "{:.}".len() && text.starts_with("{:.") && text.ends_with('}') {
                    state = State::Accepted(text);
                    if VERBOSE { eprintln!("  ==> <div>, state {state:?}"); }
                    new_events.push(Event::InlineHtml(
                        format!("<div class=\"{}\">\n\n", &text[3..text.len() - 1]).into(),
                    ));
                } else {
                    state = State::BeforeStart;
                    new_events.push(Event::Start(Tag::Paragraph));
                    new_events.push(Event::Text(text.into()));
                }
            }

            Event::End(TagEnd::Paragraph) => {
                if matches!(state, State::Expecting) {
                    new_events.push(Event::Start(Tag::Paragraph));
                }
                new_events.push(Event::End(TagEnd::Paragraph));
                if matches!(state, State::Inner) {
                    new_events.push(Event::InlineHtml("\n</div>\n\n".into()));
                }
                state = State::BeforeStart;
            }

            ev => {
                if matches!(state, State::Expecting) {
                    state = State::BeforeStart;
                    new_events.push(Event::Start(Tag::Paragraph));
                }
                new_events.push(ev);
            }
        }
    }
    if VERBOSE { eprintln!("new_events:\n{}", new_events.iter().map(|e| format!("####> {e:?}")).collect::<Vec<_>>().join("\n")); }
    let mut buf = String::with_capacity(chapter.content.capacity());
    pulldown_cmark_to_cmark::cmark(new_events.into_iter(), &mut buf).expect("can re-render cmark");
    chapter.content = buf;
    Ok(())
}

fn book_preprocessing<R: Read>(pre: &Classy, input: R) -> Result<Book, Error> {
    let (ctx, book) = mdbook_preprocessor::parse_input(input)?;

    if ctx.mdbook_version != mdbook_core::MDBOOK_VERSION {
        // We should probably use the `semver` crate to check compatibility
        // here...
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook_core::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }
    pre.run(&ctx, book)
}

/// Housekeeping:
/// 1. Check compatibility between preprocessor and mdbook
/// 2. deserialize, run the transformation, and reserialize.
fn handle_io_preprocessing(pre: &Classy) -> Result<(), Error> {
    let processed_book = book_preprocessing(pre, io::stdin())?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

/// Check to see if we support the processor (classy only supports HTML right now)
fn handle_supports(pre: &Classy, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .get_one::<String>("renderer")
        .expect("Required argument");
    let supported = pre.supports_renderer(renderer)
        .unwrap_or_else(|e| panic!("Couldn't check if renderer supported: {e}"));

    if supported {
        process::exit(0);
    }
    process::exit(1);
}

fn main() {
    // 1. Define command interface, requiring renderer to be specified.
    let matches = command!("classy")
        .about("A mdbook preprocessor that recognizes kramdown style paragraph class annotation.")
        .subcommand(
            Command::new("supports")
                .arg(arg!(<renderer>).required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
        .get_matches();

    // 2. Instantiate the preprocessor.
    let preprocessor = Classy::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    }
    if let Err(e) = handle_io_preprocessing(&preprocessor) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
