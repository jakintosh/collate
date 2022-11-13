use block::Block;
use std::{collections::HashMap, path::PathBuf};

mod block;

fn main() -> Result<(), ()> {
    let Parameters { source, output } = match parse_args() {
        Some(p) => p,
        None => panic!("Incorrect arguments."),
    };

    let mut exports = HashMap::new();
    let mut library = HashMap::new();
    for path in get_filepaths_recursive(source.into()) {
        let file = read_file(&path)?;
        let block = parse_block(&path, file)?;
        register_block(&path, &mut library, &mut exports, block)?;
    }

    Ok(())
}

fn read_file(path: &PathBuf) -> Result<String, ()> {
    std::fs::read_to_string(path.clone())
        .map_err(|_| println!("'{}': Couldn't read file", path_to_str(&path)))
}

fn parse_block(path: &PathBuf, file: String) -> Result<Block, ()> {
    file.parse()
        .map_err(|e| println!("'{}': {}", path_to_str(&path), e))
}

fn register_block(
    path: &PathBuf,
    library: &mut HashMap<String, Block>,
    exports: &mut HashMap<String, Block>,
    block: Block,
) -> Result<(), ()> {
    let name = block.name.clone();
    match library.contains_key(&name) {
        true => {
            println!("'{}': Duplicate block name ({})", path_to_str(&path), name);
            Err(())
        }
        false => {
            if block.is_exported() {
                exports.insert(name.clone(), block.clone());
            }
            library.insert(name.clone(), block);
            Ok(())
        }
    }
}

fn path_to_str(path: &PathBuf) -> &str {
    match path.to_str() {
        Some(s) => s,
        None => "<invalid path>",
    }
}

fn get_filepaths_recursive(dir: PathBuf) -> Vec<PathBuf> {
    match std::fs::read_dir(dir) {
        Ok(dir) => dir
            .filter_map(|e| e.ok())
            .filter_map(|e| match e.metadata() {
                Ok(meta) => {
                    if meta.is_dir() {
                        Some(get_filepaths_recursive(e.path()))
                    } else if meta.is_file() {
                        Some(vec![e.path()])
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .flat_map(|paths| paths.into_iter())
            .collect(),
        Err(_) => Vec::new(),
    }
}

struct Parameters {
    source: String,
    output: String,
}

fn parse_args() -> Option<Parameters> {
    let mut args = std::env::args();
    let mut map: HashMap<String, String> = HashMap::new();
    args.next(); // skip first arg, bin location
    while let Some(arg) = args.next() {
        let token = {
            if let Some(t) = arg.strip_prefix("--") {
                String::from(t)
            } else if let Some(t) = arg.strip_prefix("--") {
                String::from(t)
            } else {
                arg
            }
        };

        if let Some((key, value)) = parse_arg(&mut args, token) {
            map.insert(key, value);
        }
    }

    let source = {
        if map.contains_key("s") {
            map["s"].clone()
        } else if map.contains_key("source") {
            map["source"].clone()
        } else {
            String::from(".")
        }
    };

    let output = {
        if map.contains_key("o") {
            map["o"].clone()
        } else if map.contains_key("output") {
            map["output"].clone()
        } else {
            return None;
        }
    };

    Some(Parameters { source, output })
}

fn parse_arg(args: &mut std::env::Args, token: String) -> Option<(String, String)> {
    if token.contains("=") {
        let subtokens: Vec<&str> = token.split("=").collect();
        if subtokens.len() == 2 {
            return Some((String::from(subtokens[0]), String::from(subtokens[1])));
        }
    }

    let value = args.next()?;
    Some((token, value))
}
