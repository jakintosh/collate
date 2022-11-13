use std::str::FromStr;

const DEFINE_BLOCK_COMMAND: &str = "def-block";
const DEFINE_PARAMS_COMMAND: &str = "def-params";
const ENABLE_EXPORT_COMMAND: &str = "export";
const USE_BLOCK_COMMAND: &str = "use-block";
const USE_VALUE_COMMAND: &str = "use-param";

#[derive(Clone)]
pub(crate) struct Block {
    pub name: String,
    pub export: Option<String>,
    pub values: Vec<String>,
    pub elements: Vec<Element>,
}

pub(crate) enum Component {
    Attribute(Attribute),
    Element(Element),
}

pub(crate) enum Attribute {
    Name(String),
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
        block_name: Argument,
        parameters: Vec<Argument>,
    },
    UseValue {
        value_name: Argument,
    },
}

impl Block {
    pub(crate) fn is_exported(&self) -> bool {
        self.export.is_some()
    }
    pub(crate) fn validate(components: Vec<Component>) -> Result<Block, String> {
        let mut name = None;
        let mut export = None;
        let mut values = Vec::new();
        let mut elements = Vec::new();
        for component in components {
            match component {
                Component::Attribute(attr) => match attr {
                    Attribute::Name(n) => match name {
                        None => name = Some(n),
                        Some(_) => return Err("multiple names defined".into()),
                    },
                    Attribute::Export(e) => match export {
                        None => export = Some(e),
                        Some(_) => return Err("multiple exports defined".into()),
                    },
                    Attribute::Value(v) => match values.contains(&v) {
                        false => values.push(v),
                        true => return Err(format!("duplicate value defined: {}", v)),
                    },
                },
                Component::Element(e) => elements.push(e),
            }
        }

        match name {
            Some(name) => Ok(Block {
                name,
                export,
                values,
                elements,
            }),
            None => Err("missing required name attribute".into()),
        }
    }
}

enum ParserState {
    Content,
    CommandFlag,
    Command,
    SkipNewline,
    InvalidCommand(String),
}
impl FromStr for Block {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn block_components_from_commands(commands: Vec<&str>) -> Result<Vec<Component>, String> {
            if commands.len() < 2 {
                return Err(String::from("Not enough arguments"));
            }
            match commands[0] {
                DEFINE_BLOCK_COMMAND => {
                    let name = String::from(commands[1]);
                    let attribute = Attribute::Name(name);
                    let component = Component::Attribute(attribute);
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
                USE_BLOCK_COMMAND => {
                    let block_name = argument_from_str(commands[1]);
                    let parameters = match commands.len() {
                        len if len > 2 => commands[2..]
                            .iter()
                            .map(|p| argument_from_str(*p))
                            .collect(),
                        _ => Vec::new(),
                    };
                    let element = Element::UseBlock {
                        block_name,
                        parameters,
                    };
                    let component = Component::Element(element);
                    Ok(vec![component])
                }
                USE_VALUE_COMMAND => {
                    let value_name = argument_from_str(commands[1]);
                    let element = Element::UseValue { value_name };
                    let component = Component::Element(element);
                    Ok(vec![component])
                }
                _ => Err(String::from("Unknown command")),
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
        fn push_to_state(buffer: &mut String, c: char, state: ParserState) -> ParserState {
            buffer.push(c);
            state
        }
        fn close_content(buffer: &mut String, components: &mut Vec<Component>) -> ParserState {
            if !buffer.is_empty() {
                let content = flush(buffer);
                let element = Element::Content(content);
                let component = Component::Element(element);
                components.push(component);
            }
            ParserState::Command
        }
        fn close_command(buffer: &mut String, components: &mut Vec<Component>) -> ParserState {
            if !buffer.is_empty() {
                let command = flush(buffer);
                let commands: Vec<_> = command.split_whitespace().collect();
                match block_components_from_commands(commands) {
                    Ok(mut c) => components.append(&mut c),
                    Err(err) => return ParserState::InvalidCommand(err),
                }
            }
            ParserState::SkipNewline
        }

        let mut state = ParserState::Content;
        let mut buffer = String::with_capacity(s.len());
        let mut components = Vec::new();
        for c in s.chars() {
            state = match state {
                ParserState::Content => match c {
                    '!' => ParserState::CommandFlag,
                    _ => push_to_state(&mut buffer, c, ParserState::Content),
                },
                ParserState::CommandFlag => match c {
                    '{' => close_content(&mut buffer, &mut components),
                    '!' => push_to_state(&mut buffer, c, ParserState::CommandFlag),
                    _ => push_to_state(&mut buffer, c, ParserState::Content),
                },
                ParserState::Command => match c {
                    '}' => close_command(&mut buffer, &mut components),
                    _ => push_to_state(&mut buffer, c, ParserState::Command),
                },
                ParserState::SkipNewline => match c {
                    '\n' => ParserState::Content,
                    _ => push_to_state(&mut buffer, c, ParserState::Content),
                },
                ParserState::InvalidCommand(reason) => {
                    return Err(format!("Invalid command: {}", reason));
                }
            };
        }

        Block::validate(components)
    }
}
