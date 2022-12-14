use std::{collections::HashMap, path::PathBuf};

const HELP: &str = "
collate v1 by @jakintosh

USAGE:
`-s` or `--source` (optional, default is '.')  | dir of files to collate
`-o` or `--output` (required)                  | dir to render output

VALID ARGUMENT SYNTAX:
    `-s=dir`
    `-s dir`
    `--source=dir`
    `--source dir`";

struct Parameters {
    source: PathBuf,
    output: PathBuf,
}
impl TryFrom<std::env::Args> for Parameters {
    type Error = String;

    fn try_from(mut args: std::env::Args) -> Result<Self, Self::Error> {
        fn parse_arg(args: &mut std::env::Args, token: String) -> Option<(String, String)> {
            match token.split('=').collect::<Vec<_>>() {
                subtokens if subtokens.len() == 2 => {
                    Some((subtokens[0].into(), subtokens[1].into()))
                }
                _ => Some((token, args.next()?)),
            }
        }
        fn map_arg(
            map: &HashMap<String, String>,
            short: &str,
            long: &str,
            default: Result<String, String>,
        ) -> Result<String, String> {
            if map.contains_key(short) {
                Ok(map[short].clone())
            } else if map.contains_key(long) {
                Ok(map[long].clone())
            } else {
                default
            }
        }

        args.next(); // skip first arg, bin location

        let mut map: HashMap<String, String> = HashMap::new();
        while let Some(arg) = args.next() {
            let token = {
                if let Some(t) = arg.strip_prefix("--") {
                    String::from(t)
                } else if let Some(t) = arg.strip_prefix("-") {
                    String::from(t)
                } else {
                    arg
                }
            };

            if let Some((key, value)) = parse_arg(&mut args, token) {
                map.insert(key, value);
            }
        }

        let source = map_arg(&map, "s", "source", Ok(".".into()))?.into();
        let output = map_arg(&map, "o", "output", Err("--output param missing".into()))?.into();
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
    let library = match collate::Library::new_from_dir(&source) {
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
