use crate::block::Block;
use std::{collections::HashMap, fs, path::PathBuf};

pub struct Library {
    blocks: HashMap<String, Block>,
    exports: HashMap<String, PathBuf>,
}

impl Library {
    pub fn new() -> Library {
        Library {
            blocks: HashMap::new(),
            exports: HashMap::new(),
        }
    }

    pub fn new_from_dir(dir: &PathBuf) -> Result<Library, String> {
        let mut library = Library::new();
        for path in get_filepaths_recursive(dir) {
            library.import_from_file(&path)?;
        }
        Ok(library)
    }

    pub fn import_from_file(&mut self, path: &PathBuf) -> Result<(), String> {
        fn display(path: &PathBuf) -> &str {
            match path.to_str() {
                Some(s) => s,
                None => "<invalid path>",
            }
        }

        let file = fs::read_to_string(&path).map_err(|e| format!("'{}': {}", display(path), e))?;

        self.import_from_string(&file)
            .map_err(|e| format!("'{}': {}", display(path), e))
    }

    pub fn import_from_string(&mut self, string: &str) -> Result<(), String> {
        for block in Block::parse(string)? {
            if self.blocks.contains_key(&block.name) {
                return Err(format!("Name taken '{}'", &block.name));
            }

            if let Some(export) = &block.export {
                self.exports.insert(block.name.clone(), export.into());
            }

            self.blocks.insert(block.name.clone(), block);
        }

        Ok(())
    }

    pub fn render(&self, name: &str) -> Result<String, String> {
        let block = match self.blocks.get(name) {
            Some(b) => b,
            None => return Err(format!("Library::render(): block '{}' not found", name)),
        };
        let render = block.render(&self.blocks)?;
        Ok(render)
    }

    pub fn export_all(&self, dir: &PathBuf) -> Result<(), String> {
        ensure_directory(dir)?;
        for (block_name, file_path) in &self.exports {
            let render = self.render(block_name)?;
            let path = build_path(&dir, &file_path);
            fs::write(&path, &render).map_err(|e| format!("{}: {}", block_name, e))?;

            println!(
                "Exported block '{}' ({}B) to '{}'",
                &block_name,
                render.as_bytes().len(),
                path.to_string_lossy()
            );
        }

        Ok(())
    }
}

fn build_path(base: &PathBuf, append: &PathBuf) -> PathBuf {
    let mut path = base.clone();
    path.push(append);
    path
}

fn get_filepaths_recursive(dir: &PathBuf) -> Vec<PathBuf> {
    match fs::read_dir(dir) {
        Ok(dir) => dir
            .filter_map(|e| e.ok())
            .filter_map(|e| match e.metadata() {
                Ok(meta) => {
                    if meta.is_dir() {
                        Some(get_filepaths_recursive(&e.path()))
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

fn ensure_directory(path: &PathBuf) -> Result<(), String> {
    match path.is_dir() {
        true => Ok(()),
        false => match fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(err) => Err(format!("Couldn't create output directory: {}", err)),
        },
    }
}
