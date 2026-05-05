use std::fs;
use std::io::{self, Read, Write};
use std::process::exit;

use facet::Facet;
use facet_args as args;

use balso_graph::Builder;
use balso_parser::read;

#[derive(Facet, Debug)]
struct Args {
    #[facet(default, args::positional)]
    input: Option<String>,

    #[facet(default, args::named, args::short = 'o')]
    output: Option<String>,
}

fn main() {
    let args: Args = match args::from_std_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error parsing arguments: {:?}", e);
            exit(1);
        }
    };

    let input = match &args.input {
        Some(path) => fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {:?}", path, e);
            exit(1);
        }),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("Failed to read stdin: {:?}", e);
                exit(1);
            });
            buf
        }
    };

    let mut builder = Builder::new();
    if let Err(e) = read(&input.trim(), &mut builder) {
        eprintln!("Error parsing SMILES {}: {:?}", input.trim(), e);
        exit(1);
    }

    let svg = builder.to_svg();
    match &args.output {
        Some(path) => {
            fs::write(path, svg).unwrap_or_else(|e| {
                eprintln!("Failed to write {}: {:?}", path, e);
                exit(1);
            });
        }
        None => {
            io::stdout().write_all(svg.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Failed to write to stdout: {:?}", e);
                exit(1);
            });
        }
    }
}
