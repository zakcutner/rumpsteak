use clap::Parser;
use rumpsteak_generate::Builder;
use std::{error::Error, process::exit};

#[derive(Parser)]
#[clap(version, author, about)]
struct Arguments {
    /// Sets the name of the protocol.
    #[clap(short, long)]
    name: String,

    /// Sets the output path for generated Rust code.
    #[clap(short, long, default_value = "-")]
    output: String,

    /// Sets the input paths for Graphviz DOT files.
    inputs: Vec<String>,
}

fn generate() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();
    let mut builder = Builder::default().name(&arguments.name);

    for input in &arguments.inputs {
        builder = builder.role(input.as_str());
    }

    let protocol = builder.generate()?;
    match arguments.output.as_str() {
        "-" => print!("{}", protocol),
        path => protocol.write_to_file(path)?,
    }

    Ok(())
}

fn main() {
    if let Err(err) = generate() {
        eprintln!("{}", err);
        exit(1);
    }
}
