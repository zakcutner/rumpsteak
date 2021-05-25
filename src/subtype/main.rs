use argh::FromArgs;
use rumpsteak::{
    fsm::{dot, Fsm},
    subtype,
};
use std::{
    borrow::Cow,
    error::Error,
    fmt::Display,
    fs,
    io::{self, Write},
    process::exit,
    str::FromStr,
};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

struct ColorChoice(termcolor::ColorChoice);

impl ColorChoice {
    fn auto() -> Self {
        match atty::is(atty::Stream::Stdout) {
            true => Self(termcolor::ColorChoice::Auto),
            false => Self(termcolor::ColorChoice::Never),
        }
    }
}

impl FromStr for ColorChoice {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::auto()),
            "always" => Ok(Self(termcolor::ColorChoice::Always)),
            "never" => Ok(Self(termcolor::ColorChoice::Never)),
            _ => Err("invalid color choice, possible values are 'auto', 'always' or 'never'"),
        }
    }
}

impl From<ColorChoice> for termcolor::ColorChoice {
    fn from(ColorChoice(choice): ColorChoice) -> Self {
        choice
    }
}

/// Compares two FSMs in DOT format to check if the left is a subtype of the
/// right.
#[derive(FromArgs)]
struct Options {
    /// whether to use colored output, defaults to 'auto'
    #[argh(option, default = "ColorChoice::auto()")]
    color: ColorChoice,

    /// how many visits to allow to each state
    #[argh(option)]
    visits: usize,

    #[argh(positional)]
    left: String,

    #[argh(positional)]
    right: String,
}

fn error(message: impl Display, err: impl Error) -> ! {
    eprintln!("{}: {}\n", message, err);
    exit(1)
}

fn read_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            let err = io::Error::from(err.kind());
            error(format_args!("Error opening '{}'", path), err);
        }
    }
}

fn parse_fsm<'a>(input: &'a str, path: &str) -> Fsm<Cow<'a, str>, Cow<'a, str>> {
    match dot::parse(input) {
        Ok(fsm) => fsm,
        Err(err) => error(format_args!("Error parsing '{}'", path), err),
    }
}

fn set_color(mut stream: impl WriteColor, color: Color) -> io::Result<()> {
    stream.set_color(ColorSpec::new().set_fg(Some(color)))
}

fn main() {
    let options = argh::from_env::<Options>();

    let left = read_file(&options.left);
    let left = parse_fsm(&left, &options.left);

    let right = read_file(&options.right);
    let right = parse_fsm(&right, &options.right);

    let is_subtype = subtype::is_subtype(&left, &right, options.visits);

    let mut stdout = StandardStream::stdout(options.color.into());
    write!(&mut stdout, "left ").unwrap();

    match is_subtype {
        true => {
            set_color(&mut stdout, Color::Green).unwrap();
            write!(&mut stdout, "IS").unwrap();
        }
        false => {
            set_color(&mut stdout, Color::Red).unwrap();
            write!(&mut stdout, "IS NOT").unwrap();
        }
    }

    stdout.reset().unwrap();
    writeln!(&mut stdout, " a subtype of right\n").unwrap();
}
