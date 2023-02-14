use std::path::PathBuf;

const HELP: &str = "
collate v1 by @jakintosh

USAGE:
    collate <source_dir> <output_dir>";

struct Parameters {
    source: PathBuf,
    output: PathBuf,
}
impl TryFrom<std::env::Args> for Parameters {
    type Error = String;

    fn try_from(mut args: std::env::Args) -> Result<Self, Self::Error> {
        args.next(); // skip first arg, bin location

        let Some(source) = args.next() else {
            return Err(String::from("Missing `source_dir` argument"));
        };
        let Some(output) = args.next() else {
            return Err(String::from("Missing `output_dir` argument"));
        };

        let source = source.into();
        let output = output.into();

        Ok::<Parameters, String>(Parameters { source, output })
    }
}

fn main() {
    let Parameters { source, output } = match std::env::args().try_into() {
        Ok(params) => params,
        Err(err) => {
            println!("{}{}", err, HELP);
            return;
        }
    };
    let mut library = match collate::Library::new_from_dir(&source) {
        Ok(l) => l,
        Err(err) => {
            println!("Failed to build library: {}", err);
            return;
        }
    };
    match library.export_all(&output) {
        Ok(_) => {}
        Err(err) => println!("Failed to export: {}", err),
    };
}
