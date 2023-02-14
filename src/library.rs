use crate::block::{Block, Export};
use std::{collections::HashMap, fs, path::PathBuf};

pub struct Library {
    blocks: HashMap<String, Block>,
    block_exports: Vec<String>,
    file_exports: HashMap<String, PathBuf>,
}

impl Library {
    pub fn new() -> Library {
        Library {
            blocks: HashMap::new(),
            block_exports: Vec::new(),
            file_exports: HashMap::new(),
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
                match export {
                    Export::Block => {
                        self.block_exports.push(block.name.clone());
                    }
                    Export::File(path) => {
                        self.file_exports.insert(block.name.clone(), path.into());
                    }
                }
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
        let render = block
            .render(&self.blocks)
            .map_err(|e| format!("Library::render(): block '{}': {}", name, e))?;
        Ok(render)
    }

    pub fn export_all(&mut self, dir: &PathBuf) -> Result<(), String> {
        loop {
            // clone list of block exports and ingest
            let block_exports = self.block_exports.clone();
            self.block_exports.clear();
            for block_name in block_exports {
                let render = self.render(&block_name)?;
                println!("Rendered {} to:\n\n{}", block_name, render);
                self.import_from_string(&render)?;
            }

            // if we haven't made more block exports, exit
            match self.block_exports.is_empty() {
                true => break,
                false => continue,
            }
        }

        for (block_name, file_path) in &self.file_exports {
            let render = self.render(block_name)?;
            let path = build_path(&dir, &file_path);
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("{}", e))?;
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
