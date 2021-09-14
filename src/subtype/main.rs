use argh::FromArgs;
use rumpsteak::{
    fsm::{
        dot::{self, ParseErrors},
        Fsm,
    },
    subtype,
};
use std::{
    convert::Infallible,
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

fn unwrap_fsm<R, N, E>(fsm: Result<Fsm<R, N, E>, ParseErrors>, path: &str) -> Fsm<R, N, E> {
    match fsm {
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
    let left = dot::parse(&left);

    let right = read_file(&options.right);
    let right = dot::parse(&right);

    let mut stdout = StandardStream::stdout(options.color.into());
    for (i, (left, right)) in left.zip(right).enumerate() {
        let left = unwrap_fsm::<String, String, Infallible>(left, &options.left);
        let right = unwrap_fsm(right, &options.right);

        let is_subtype = subtype::is_subtype(&left, &right, options.visits);
        write!(&mut stdout, "{}. left ", i + 1).unwrap();

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
        writeln!(&mut stdout, " a subtype of right").unwrap();
    }

    writeln!(&mut stdout).unwrap();
}
