use std::path::PathBuf;

const HELP: &str = "
collate v1 by @jakintosh

USAGE:
    collate <source_dir> <output_dir> [--verbose | --quiet]";

const VERSION: &str = "
collate v1 by @jakintosh";

enum Parameters {
    Run {
        source: PathBuf,
        output: PathBuf,
        quiet: bool,
        verbose: bool,
    },
    Help,
    Version,
}
impl TryFrom<std::env::Args> for Parameters {
    type Error = String;

    fn try_from(mut args: std::env::Args) -> Result<Self, Self::Error> {
        args.next(); // skip first arg, bin location

        let source = match args.next() {
            Some(arg) => match arg.as_str() {
                "--help" | "-h" => return Ok(Parameters::Help),
                "--version" => return Ok(Parameters::Version),
                _ => arg,
            },
            None => return Err(String::from("Missing `source_dir` argument")),
        };
        let output = match args.next() {
            Some(arg) => match arg.as_str() {
                "--help" | "-h" => return Ok(Parameters::Help),
                "--version" => return Ok(Parameters::Version),
                _ => arg,
            },
            None => return Err(String::from("Missing `output_dir` argument")),
        };
        let source = source.into();
        let output = output.into();

        let mut quiet = false;
        let mut verbose = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--quiet" | "-q" => quiet = true,
                "--verbose" | "-v" => verbose = true,
                _ => panic!("unrecognized parameter: {}\n{}", arg, HELP),
            }
        }

        Ok::<Parameters, String>(Parameters::Run {
            source,
            output,
            quiet,
            verbose,
        })
    }
}

fn main() {
    let parameters: Parameters = match std::env::args().try_into() {
        Ok(params) => params,
        Err(e) => {
            println!("{}\n{}", e, HELP);
            return;
        }
    };
    let (source, output, quiet, verbose) = match parameters {
        Parameters::Run {
            source,
            output,
            quiet,
            verbose,
        } => (source, output, quiet, verbose),
        Parameters::Help => {
            println!("{}", HELP);
            return;
        }
        Parameters::Version => {
            println!("{}", VERSION);
            return;
        }
    };
    let mut library = match collate::Library::new_from_dir(&source) {
        Ok(l) => l,
        Err(err) => {
            println!("Parsing failed: {}", err);
            return;
        }
    };
    match library.export_all(&output, verbose) {
        Ok(_) => {}
        Err(err) => println!("Export failed: {}", err),
    };
}
