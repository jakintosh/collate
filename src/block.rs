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
    pub elements: Vec<BlockElement>,
}
impl Block {
    pub(crate) fn is_exported(&self) -> bool {
        self.export.is_some()
    }
    pub(crate) fn validate(components: Vec<BlockComponent>) -> Result<Block, String> {
        let mut name = None;
        let mut export = None;
        let mut values = Vec::new();
        let mut elements = Vec::new();
        for component in components {
            match component {
                BlockComponent::Attribute(attr) => match attr {
                    BlockAttribute::Name(n) => match name {
                        None => name = Some(n),
                        Some(_) => return Err("multiple names defined".into()),
                    },
                    BlockAttribute::Export(e) => match export {
                        None => export = Some(e),
                        Some(_) => return Err("multiple exports defined".into()),
                    },
                    BlockAttribute::Value(v) => match values.contains(&v) {
                        false => values.push(v),
                        true => return Err(format!("duplicate value defined: {}", v)),
                    },
                },
                BlockComponent::Element(e) => elements.push(e),
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
pub(crate) enum BlockComponent {
    Attribute(BlockAttribute),
    Element(BlockElement),
}
pub(crate) enum BlockAttribute {
    Name(String),
    Export(String),
    Value(String),
}

#[derive(Clone)]
pub(crate) enum BlockElement {
    Content(String),
    UseBlock {
        block_name: Argument,
        parameters: Vec<Argument>,
    },
    UseValue {
        value_name: Argument,
    },
}

#[derive(Clone)]
pub(crate) enum Argument {
    Literal(String),
    Value(String),
}

fn block_component_from_parsed_element(e: &ParsedElement) -> Result<Vec<BlockComponent>, String> {
    fn argument_from_str(s: &str) -> Argument {
        match s.strip_prefix('#') {
            Some(s) => Argument::Value(String::from(s)),
            None => Argument::Literal(String::from(s)),
        }
    }
    fn block_components_from_commands(commands: Vec<&str>) -> Option<Vec<BlockComponent>> {
        if commands.len() < 2 {
            return None;
        }
        match commands[0] {
            DEFINE_BLOCK_COMMAND => {
                let name = String::from(commands[1]);
                let attribute = BlockAttribute::Name(name);
                let component = BlockComponent::Attribute(attribute);
                Some(vec![component])
            }
            DEFINE_PARAMS_COMMAND => {
                let num_params = commands.len() - 1;
                let mut components = Vec::with_capacity(num_params);
                for i in 1..(1 + num_params) {
                    let param_name = String::from(commands[i]);
                    let attribute = BlockAttribute::Value(param_name);
                    let component = BlockComponent::Attribute(attribute);
                    components.push(component);
                }
                Some(components)
            }
            ENABLE_EXPORT_COMMAND => {
                let export_path = String::from(commands[1]);
                let attribute = BlockAttribute::Export(export_path);
                let component = BlockComponent::Attribute(attribute);
                Some(vec![component])
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
                let element = BlockElement::UseBlock {
                    block_name,
                    parameters,
                };
                let component = BlockComponent::Element(element);
                Some(vec![component])
            }
            USE_VALUE_COMMAND => {
                let value_name = argument_from_str(commands[1]);
                let element = BlockElement::UseValue { value_name };
                let component = BlockComponent::Element(element);
                Some(vec![component])
            }
            _ => None,
        }
    }

    let components = match e {
        ParsedElement::Content(string) => {
            let content = string.clone();
            let element = BlockElement::Content(content);
            let component = BlockComponent::Element(element);
            vec![component]
        }
        ParsedElement::Command(string) => {
            let commands: Vec<_> = string.split_whitespace().collect();
            match block_components_from_commands(commands) {
                Some(components) => components,
                None => return Err(String::from("Invalid command!")),
            }
        }
    };

    Ok(components)
}

enum ParserState {
    Content,
    CommandFlag,
    Command,
    SkipNewline,
}
enum ParsedElement {
    Content(String),
    Command(String),
}
impl FromStr for Block {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn push_to_state(builder: &mut String, c: char, state: ParserState) -> ParserState {
            builder.push(c);
            state
        }
        fn close_content(builder: &mut String, elements: &mut Vec<ParsedElement>) -> ParserState {
            if !builder.is_empty() {
                let content = ParsedElement::Content(builder.clone());
                builder.clear();
                elements.push(content);
            }
            ParserState::Command
        }
        fn close_command(builder: &mut String, elements: &mut Vec<ParsedElement>) -> ParserState {
            if !builder.is_empty() {
                let command = ParsedElement::Command(builder.clone());
                builder.clear();
                elements.push(command);
            }
            ParserState::SkipNewline
        }

        let mut state = ParserState::Content;
        let mut builder = String::with_capacity(s.len());
        let mut elements = Vec::new();
        for c in s.chars() {
            state = match state {
                ParserState::Content => match c {
                    '!' => ParserState::CommandFlag,
                    _ => push_to_state(&mut builder, c, ParserState::Content),
                },
                ParserState::CommandFlag => match c {
                    '{' => close_content(&mut builder, &mut elements),
                    _ => push_to_state(&mut builder, c, ParserState::Content),
                },
                ParserState::Command => match c {
                    '}' => close_command(&mut builder, &mut elements),
                    _ => push_to_state(&mut builder, c, ParserState::Command),
                },
                ParserState::SkipNewline => match c {
                    '\n' => ParserState::Content,
                    _ => push_to_state(&mut builder, c, ParserState::Content),
                },
            };
        }

        let components = elements
            .iter()
            .filter_map(|e| block_component_from_parsed_element(e).ok())
            .flat_map(|c| c.into_iter())
            .collect();

        Block::validate(components)
    }
}
