use block::Block;
use std::{collections::HashMap, path::PathBuf};

mod block;

fn main() -> Result<(), String> {
    let Parameters { source, output } = parse_args()?;

    let mut library = HashMap::new();
    let mut exports = HashMap::new();
    for path in get_filepaths_recursive(source) {
        parse_block_and_register(&path, &mut library, &mut exports)?;
    }
    for (block_name, file_path) in exports {
        let block = library.get(&block_name).expect("library/export mismatch");
        let render = block.render(&library)?;
        let path = build_path(&output, &file_path);
        std::fs::write(&path, &render).map_err(|e| format!("{}: {}", block_name, e))?;
        println!(
            "Exported block '{}' ({}B) to '{}'",
            &block_name,
            render.as_bytes().len(),
            path.to_string_lossy()
        );
    }

    Ok(())
}

fn build_path(base: &PathBuf, append: &PathBuf) -> PathBuf {
    let mut path = base.clone();
    path.push(append);
    path
}

fn parse_block_and_register(
    path: &PathBuf,
    library: &mut HashMap<String, Block>,
    exports: &mut HashMap<String, PathBuf>,
) -> Result<(), String> {
    let display_path = match path.to_str() {
        Some(s) => s,
        None => "<invalid path>",
    };
    let file = match std::fs::read_to_string(&path) {
        Ok(f) => f,
        Err(e) => return Err(format!("'{}': {}", display_path, e)),
    };
    let block = match file.parse::<Block>() {
        Ok(b) => b,
        Err(e) => return Err(format!("'{}': {}", display_path, e)),
    };
    let result = match library.contains_key(&block.name) {
        true => Err(format!("'{}': Name taken '{}'", display_path, &block.name)),
        false => {
            if let Some(export) = &block.export {
                exports.insert(block.name.clone(), export.into());
            }
            library.insert(block.name.clone(), block);

            Ok(())
        }
    };

    result
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
    source: PathBuf,
    output: PathBuf,
}

fn parse_args() -> Result<Parameters, String> {
    fn parse_arg(args: &mut std::env::Args, token: String) -> Option<(String, String)> {
        match token.split('=').collect::<Vec<_>>() {
            subtokens if subtokens.len() == 2 => Some((subtokens[0].into(), subtokens[1].into())),
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

    let mut args = std::env::args();
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

    Ok(Parameters { source, output })
}
