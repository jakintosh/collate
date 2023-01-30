use std::collections::HashMap;

const COMMAND_FLAG: char = '^';
const COMMAND_START: char = '|';
const COMMAND_END: char = '|';

const NEW_BLOCK_COMMAND: &str = "n";
const DEFINE_PARAMS_COMMAND: &str = "p";
const ENABLE_EXPORT_COMMAND: &str = "x";
const USE_BLOCK_COMMAND: &str = "u";
const USE_BLOCK_INDENTED_COMMAND: &str = "ui";
const END_BLOCK_COMMAND: &str = "e";

#[derive(Clone)]
pub(crate) struct Block {
    pub name: String,
    pub export: Option<String>,
    pub values: Vec<String>,
    pub elements: Vec<Element>,
}

pub(crate) enum Component {
    Open { name: String },
    Attribute(Attribute),
    Element(Element),
    Close,
}

pub(crate) enum Attribute {
    Export(String),
    Value(String),
}

#[derive(Clone)]
pub(crate) enum Argument {
    Literal(String),
    Value(String),
}

#[derive(Clone)]
pub(crate) enum Element {
    Content(String),
    UseBlock {
        indented: bool,
        block_name: Argument,
        parameters: Option<Vec<Argument>>,
    },
}

impl Block {
    pub(crate) fn parse(string: &str) -> Result<Vec<Block>, String> {
        enum State {
            Content,
            CommandFlag,
            Command,
            SkipNewline,
            CancelledFlag,
            InvalidCommand(String),
        }
        fn block_components_from_commands(commands: Vec<&str>) -> Result<Vec<Component>, String> {
            match commands[0] {
                END_BLOCK_COMMAND => Ok(vec![Component::Close]),
                first => {
                    if commands.len() < 2 {
                        return Err(String::from("Not enough arguments"));
                    }
                    match first {
                        NEW_BLOCK_COMMAND => {
                            let name = String::from(commands[1]);
                            let component = Component::Open { name };
                            Ok(vec![component])
                        }
                        DEFINE_PARAMS_COMMAND => {
                            let num_params = commands.len() - 1;
                            let mut components = Vec::with_capacity(num_params);
                            for i in 1..(1 + num_params) {
                                let param_name = String::from(commands[i]);
                                let attribute = Attribute::Value(param_name);
                                let component = Component::Attribute(attribute);
                                components.push(component);
                            }
                            Ok(components)
                        }
                        ENABLE_EXPORT_COMMAND => {
                            let export_path = String::from(commands[1]);
                            let attribute = Attribute::Export(export_path);
                            let component = Component::Attribute(attribute);
                            Ok(vec![component])
                        }
                        USE_BLOCK_COMMAND | USE_BLOCK_INDENTED_COMMAND => {
                            let indented = match first {
                                USE_BLOCK_COMMAND => false,
                                USE_BLOCK_INDENTED_COMMAND => true,
                                _ => unreachable!(),
                            };
                            let block_name = argument_from_str(commands[1]);
                            let parameters = match commands.len() {
                                len if len > 2 => Some(
                                    commands[2..]
                                        .iter()
                                        .map(|p| argument_from_str(*p))
                                        .collect(),
                                ),
                                _ => None,
                            };
                            let element = Element::UseBlock {
                                indented,
                                block_name,
                                parameters,
                            };
                            let component = Component::Element(element);
                            Ok(vec![component])
                        }
                        cmd => Err(format!("Unknown command '{}'", cmd)),
                    }
                }
            }
        }
        fn argument_from_str(s: &str) -> Argument {
            match s.strip_prefix('#') {
                Some(s) => Argument::Value(String::from(s)),
                None => Argument::Literal(String::from(s)),
            }
        }
        fn flush(buffer: &mut String) -> String {
            let contents = buffer.clone();
            buffer.clear();
            contents
        }
        fn push_to_state(buffer: &mut String, c: char, state: State) -> State {
            buffer.push(c);
            state
        }
        fn close_content(buffer: &mut String, components: &mut Vec<Component>) -> State {
            if !buffer.is_empty() {
                let content = flush(buffer);
                let element = Element::Content(content);
                let component = Component::Element(element);
                components.push(component);
            }
            State::Command
        }
        fn close_command(buffer: &mut String, components: &mut Vec<Component>) -> State {
            if !buffer.is_empty() {
                let command = flush(buffer);
                let commands: Vec<_> = command.split_whitespace().collect();
                match block_components_from_commands(commands) {
                    Ok(mut c) => components.append(&mut c),
                    Err(err) => return State::InvalidCommand(err),
                }
            }

            // don't skip newline after 'use' commands
            match components.last() {
                Some(Component::Element(Element::UseBlock { .. })) => State::Content,
                _ => State::SkipNewline,
            }
        }

        let mut line = 1;
        let mut col = 0;
        let mut state = State::Content;
        let mut buffer = String::with_capacity(string.len());
        let mut components = Vec::new();
        for c in string.chars() {
            if c == '\n' {
                line += 1;
                col = 0;
            }
            col += 1;
            state = match state {
                State::Content => match c {
                    COMMAND_FLAG => State::CommandFlag,
                    _ => push_to_state(&mut buffer, c, State::Content),
                },
                State::CommandFlag => match c {
                    COMMAND_START => close_content(&mut buffer, &mut components),
                    COMMAND_FLAG => push_to_state(&mut buffer, c, State::CancelledFlag),
                    _ => push_to_state(&mut buffer, c, State::Content),
                },
                State::Command => match c {
                    COMMAND_END => close_command(&mut buffer, &mut components),
                    _ => push_to_state(&mut buffer, c, State::Command),
                },
                State::SkipNewline => match c {
                    '\n' => State::Content,
                    _ => push_to_state(&mut buffer, c, State::Content),
                },
                State::CancelledFlag => match c {
                    COMMAND_FLAG => push_to_state(&mut buffer, c, State::CancelledFlag),
                    _ => push_to_state(&mut buffer, c, State::Content),
                },
                State::InvalidCommand(reason) => {
                    return Err(format!("Invalid command ({}:{}): {}", line, col, reason));
                }
            };
        }
        close_content(&mut buffer, &mut components);

        Block::build(components)
    }
    pub(crate) fn build(components: Vec<Component>) -> Result<Vec<Block>, String> {
        let mut blocks = Vec::new();
        let mut components = components.into_iter();
        let name;
        loop {
            match components.next() {
                Some(Component::Open { name: n }) => {
                    name = n;
                    break;
                }
                None => return Ok(blocks), // iterator is empty
                _ => continue,
            }
        }
        let mut export = None;
        let mut values = Vec::new();
        let mut elements = Vec::new();
        while let Some(component) = components.next() {
            match component {
                Component::Open { name } => {
                    return Err(format!("Illegally nested block '{}'", name))
                }
                Component::Attribute(attr) => match attr {
                    Attribute::Export(e) => match export {
                        None => export = Some(e),
                        Some(_) => return Err("Multiple exports defined".into()),
                    },
                    Attribute::Value(v) => match values.contains(&v) {
                        false => values.push(v),
                        true => return Err(format!("Duplicate value defined: {}", v)),
                    },
                },
                Component::Element(e) => elements.push(e),
                Component::Close => {
                    // remove the final newline before the close command
                    if let Some(Element::Content(last)) = elements.last_mut() {
                        if last.ends_with('\n') {
                            last.truncate(last.len() - 1);
                        }
                    }
                    break;
                }
            }
        }

        blocks.push(Block {
            name,
            export,
            values,
            elements,
        });
        blocks.append(&mut Block::build(components.collect())?);

        Ok(blocks)
    }
    pub(crate) fn render(&self, library: &HashMap<String, Block>) -> Result<String, String> {
        self.render_with_params(library, None, 0)
    }
    fn render_with_params(
        &self,
        library: &HashMap<String, Block>,
        params: Option<Vec<String>>,
        indentation: usize,
    ) -> Result<String, String> {
        fn build_params(
            values: Vec<String>,
            params: Vec<String>,
        ) -> Result<HashMap<String, String>, String> {
            match values.len() == params.len() {
                true => {
                    let zip = values.into_iter().zip(params.into_iter());
                    let params = HashMap::from_iter(zip);
                    Ok(params)
                }
                false => Err(format!(
                    "Expected {} parameter(s), received {}",
                    values.len(),
                    params.len()
                )),
            }
        }
        fn evaluate(arg: &Argument, params: &HashMap<String, String>) -> Result<String, String> {
            match arg {
                Argument::Literal(s) => Ok(s.to_owned()),
                Argument::Value(name) => match params.get(name) {
                    Some(value) => Ok(value.clone()),
                    None => Err(format!("Value named {} does not exist", name)),
                },
            }
        }

        let params = match params {
            Some(p) => build_params(self.values.clone(), p)?,
            None => HashMap::new(),
        };

        let mut nested_indent = 0;
        let mut buffer = String::new();
        for element in &self.elements {
            let s = match element {
                Element::Content(content) => {
                    // get indentation of current line
                    let split: Vec<_> = content.split('\n').collect();
                    if split.len() > 1 {
                        nested_indent = 0;
                        let mut line = *split.last().unwrap();
                        while let Some(line_stripped) = line.strip_prefix('\t') {
                            line = line_stripped;
                            nested_indent += 1;
                        }
                    }

                    // apply indentation to the content
                    let content = match indentation {
                        0 => content.clone(),
                        _ => {
                            let mut new_content = String::new();
                            for c in content.chars() {
                                match c {
                                    '\n' => {
                                        new_content.push('\n');
                                        for _ in 0..indentation {
                                            new_content.push('\t');
                                        }
                                    }
                                    _ => new_content.push(c),
                                }
                            }
                            new_content
                        }
                    };

                    // return the content string
                    content
                }
                Element::UseBlock {
                    indented,
                    block_name,
                    parameters,
                } => {
                    let block_name = evaluate(block_name, &params)?;
                    let block = match library.get(&block_name) {
                        Some(b) => b,
                        None => return Err(format!("Using unregisterd block '{}'", block_name)),
                    };
                    let parameters: Option<Vec<String>> = match parameters {
                        Some(p) => {
                            let evaluated_params: Result<Vec<String>, String> =
                                p.iter().map(|p| evaluate(p, &params)).collect();
                            Some(evaluated_params?)
                        }
                        None => None,
                    };
                    let indentation = match indented {
                        true => indentation + nested_indent,
                        false => 0,
                    };

                    block.render_with_params(library, parameters, indentation)?
                }
            };
            buffer.push_str(&s);
        }

        Ok(buffer)
    }
}
