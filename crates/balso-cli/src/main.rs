use std::path::PathBuf;
use std::process;

use balso::{DrawOptions, SvgDrawer, ThemeManager, ThemeName};
use balso_graph::Builder;
use balso_layout::Layout;
use balso_parser::read;
use clap::{Parser, ValueEnum};

/// CLI for Balsa molecular line notation - Draw molecules from SMILES
#[derive(Parser, Debug)]
#[command(name = "balso")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input SMILES string or file (use - for stdin)
    #[arg(short, long)]
    input: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "svg")]
    format: OutputFormat,

    /// Theme to use
    #[arg(short, long, value_enum, default_value = "light")]
    theme: ThemeArg,

    /// Bond thickness in pixels
    #[arg(long, default_value = "2.0")]
    bond_thickness: f64,

    /// Bond length in pixels
    #[arg(long, default_value = "20.0")]
    bond_length: f64,

    /// Bond spacing for double/triple bonds
    #[arg(long, default_value = "4.0")]
    bond_spacing: f64,

    /// Atom font size
    #[arg(long, default_value = "14.0")]
    atom_font_size: f64,

    /// Draw aromaticity circles
    #[arg(long, default_value = "true")]
    draw_aromaticity: bool,

    /// Draw explicit hydrogens
    #[arg(long, default_value = "false")]
    draw_explicit_hydrogens: bool,

    /// Draw terminal carbons
    #[arg(long, default_value = "false")]
    draw_terminal_carbon: bool,

    /// Debug mode (shows atom indices)
    #[arg(long, default_value = "false")]
    debug: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    /// SVG output
    Svg,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ThemeArg {
    Light,
    Dark,
    HighContrast,
    Colorful,
}

impl From<ThemeArg> for ThemeName {
    fn from(val: ThemeArg) -> Self {
        match val {
            ThemeArg::Light => ThemeName::Light,
            ThemeArg::Dark => ThemeName::Dark,
            ThemeArg::HighContrast => ThemeName::HighContrast,
            ThemeArg::Colorful => ThemeName::Colorful,
        }
    }
}

fn read_input(arg: &Option<String>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    match arg {
        Some(s) if s == "-" => {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            Ok(input.trim().to_string())
        }
        Some(s) if PathBuf::from(s).exists() => Ok(std::fs::read_to_string(s)?),
        Some(s) => Ok(s.clone()),
        None => {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            Ok(input.trim().to_string())
        }
    }
}

fn main() {
    let args = Args::parse();

    // Read input SMILES
    let smiles = match read_input(&args.input) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            eprintln!("Error: No input provided");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            process::exit(1);
        }
    };

    // Parse SMILES
    let mut builder = Builder::new();
    if let Err(e) = read(&smiles, &mut builder) {
        eprintln!("Error parsing SMILES: {:?}", e);
        process::exit(1);
    }

    let atoms = builder.build();
    if atoms.is_empty() {
        eprintln!("Error: No atoms parsed from SMILES");
        process::exit(1);
    }

    // Layout
    let mut layout = Layout::new(&atoms);
    let graph = layout.layout();

    // Create draw options
    let theme_name: ThemeName = args.theme.into();
    let options = DrawOptions::default()
        .with_bond_thickness(args.bond_thickness)
        .with_bond_length(args.bond_length)
        .with_bond_spacing(args.bond_spacing)
        .with_atom_font_size(args.atom_font_size)
        .with_draw_aromaticity(args.draw_aromaticity)
        .with_draw_explicit_hydrogens(args.draw_explicit_hydrogens)
        .with_draw_terminal_carbon(args.draw_terminal_carbon)
        .with_debug(args.debug)
        .with_theme(theme_name);

    let theme = ThemeManager::with_theme(theme_name);
    let svg = SvgDrawer::new(graph, options, theme.current_theme().clone()).draw();

    // Output
    match args.output {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, svg) {
                eprintln!("Error writing to file {}: {}", path.display(), e);
                process::exit(1);
            }
            eprintln!("Wrote SVG to {}", path.display());
        }
        None => {
            println!("{}", svg);
        }
    }
}
