use std::{collections::HashMap, str::Chars};

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
    pub param_names: Vec<String>,
    pub export: Option<String>,
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
    ParamName(String),
}

#[derive(Clone)]
pub(crate) enum Argument {
    Literal(String),
    Name(String),
    ParamName(String),
}

#[derive(Clone)]
pub(crate) enum Parameter {
    Name(String),
    Literal(String),
}

#[derive(Clone)]
pub(crate) enum Element {
    Content(String),
    UseBlock {
        indented: bool,
        target: Argument,
        arguments: Option<Vec<Argument>>,
    },
}

#[derive(Clone)]
pub(crate) enum Command {
    Flag(String),
    Argument(Argument),
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
        fn commands_from_str(command_str: &str) -> Result<Vec<Command>, String> {
            fn read_word(buffer: &mut String, chars: &mut Chars) {
                while let Some(c) = chars.next() {
                    match c {
                        c if c.is_whitespace() => break,
                        c => buffer.push(c),
                    }
                }
            }
            let mut chars = command_str.chars();
            let mut commands = Vec::new();
            let mut buffer = String::new();

            // get flag
            read_word(&mut buffer, &mut chars);
            if buffer.is_empty() {
                return Err(format!("Couldn't parse command flag from {}", command_str));
            }
            commands.push(Command::Flag(buffer.clone()));
            buffer.clear();

            // get arguments
            while let Some(c) = chars.next() {
                match c {
                    '#' => {
                        read_word(&mut buffer, &mut chars);
                        commands.push(Command::Argument(Argument::ParamName(buffer.clone())));
                        buffer.clear();
                    }
                    '(' => {
                        while let Some(c) = chars.next() {
                            match c {
                                ')' => break,
                                c => buffer.push(c),
                            }
                        }
                        commands.push(Command::Argument(Argument::Literal(buffer.clone())));
                        buffer.clear();
                    }
                    c => {
                        buffer.push(c);
                        read_word(&mut buffer, &mut chars);
                        if !buffer.is_empty() {
                            commands.push(Command::Argument(Argument::Name(buffer.clone())));
                            buffer.clear();
                        }
                    }
                }
            }
            Ok(commands)
        }
        fn block_components_from_commands(
            commands: Vec<Command>,
        ) -> Result<Vec<Component>, String> {
            let mut commands = commands.into_iter();
            match commands.next() {
                Some(Command::Flag(flag)) => match flag.as_str() {
                    NEW_BLOCK_COMMAND => match commands.next() {
                        Some(Command::Argument(Argument::Name(name))) => {
                            Ok(vec![Component::Open { name }])
                        }
                        _ => Err(format!("New block command must provide a name")),
                    },
                    DEFINE_PARAMS_COMMAND => {
                        let mut components = Vec::new();
                        while let Some(next) = commands.next() {
                            match next {
                                Command::Argument(Argument::Name(name)) => {
                                    let attribute = Attribute::ParamName(name);
                                    let component = Component::Attribute(attribute);
                                    components.push(component);
                                }
                                _ => {
                                    return Err(format!(
                                        "Define params can only handle Argument::Name commands"
                                    ))
                                }
                            }
                        }
                        Ok(components)
                    }
                    ENABLE_EXPORT_COMMAND => match commands.next() {
                        Some(Command::Argument(Argument::Name(path))) => {
                            let attribute = Attribute::Export(path);
                            let component = Component::Attribute(attribute);
                            Ok(vec![component])
                        }
                        _ => {
                            return Err(format!(
                                "Enable Expost can only handle Argument::Name commands"
                            ))
                        }
                    },
                    USE_BLOCK_COMMAND | USE_BLOCK_INDENTED_COMMAND => {
                        let indented = match flag.as_str() {
                            USE_BLOCK_COMMAND => false,
                            USE_BLOCK_INDENTED_COMMAND => true,
                            _ => unreachable!(),
                        };
                        let target = match commands.next() {
                            Some(Command::Argument(arg)) => arg,
                            _ => {
                                return Err(format!(
                                    "Use block expects first command to be argument"
                                ))
                            }
                        };
                        let arguments: Vec<Argument> = commands
                            .filter_map(|c| match c {
                                Command::Argument(arg) => Some(arg),
                                Command::Flag(_) => None,
                            })
                            .collect();
                        let arguments = match arguments.is_empty() {
                            false => Some(arguments),
                            true => None,
                        };
                        let element = Element::UseBlock {
                            indented,
                            target,
                            arguments,
                        };
                        let component = Component::Element(element);
                        Ok(vec![component])
                    }
                    END_BLOCK_COMMAND => Ok(vec![Component::Close]),
                    _ => Err(format!("Unknown Command::Flag '{}'", flag)),
                },
                Some(_) => Err(format!("First command must be a flag")),
                None => Err(format!("Cannot build block from empty command list")),
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
                let commands = match commands_from_str(&command) {
                    Ok(c) => c,
                    Err(e) => panic!("{}", e),
                };
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
                _ => continue,             // skip all possible components until we hit an open
            }
        }

        let mut export = None;
        let mut param_names = Vec::new();
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
                    Attribute::ParamName(v) => match param_names.contains(&v) {
                        false => param_names.push(v),
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
            param_names,
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
        params: Option<Vec<Parameter>>,
        indentation: usize,
    ) -> Result<String, String> {
        fn build_params(
            values: Vec<String>,
            params: Vec<Parameter>,
        ) -> Result<HashMap<String, Parameter>, String> {
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
        fn evaluate(
            arg: &Argument,
            params: &HashMap<String, Parameter>,
        ) -> Result<Parameter, String> {
            match arg {
                Argument::Literal(lit) => Ok(Parameter::Literal(lit.to_owned())),
                Argument::Name(name) => Ok(Parameter::Name(name.to_owned())),
                Argument::ParamName(name) => match params.get(name) {
                    Some(param) => Ok(param.clone()),
                    None => Err(format!("Param named {} does not exist", name)),
                },
            }
        }

        let params = match params {
            Some(p) => build_params(self.param_names.clone(), p)?,
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
                    target,
                    arguments,
                } => {
                    let target_param = evaluate(target, &params)?;
                    match target_param {
                        Parameter::Literal(literal) => literal,
                        Parameter::Name(name) => {
                            let block = match library.get(&name) {
                                Some(b) => b,
                                None => return Err(format!("Using unregistered block '{}'", name)),
                            };
                            let parameters: Option<Vec<Parameter>> = match arguments {
                                Some(p) => {
                                    let evaluated_params: Result<Vec<Parameter>, String> =
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
                    }
                }
            };
            buffer.push_str(&s);
        }

        Ok(buffer)
    }
}
