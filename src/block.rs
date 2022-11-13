use std::str::FromStr;

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
                    BlockAttribute::Export(e) => match name {
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
        block_name: String,
        parameters: String,
    },
    UseValue {
        value_name: String,
    },
}
impl TryFrom<&ParsedElement> for BlockComponent {
    type Error = String;

    fn try_from(e: &ParsedElement) -> Result<Self, Self::Error> {
        let component = match e {
            ParsedElement::Content(string) => {
                BlockComponent::Element(BlockElement::Content(string.clone()))
            }
            ParsedElement::Command(string) => {
                let commands: Vec<_> = string.split_whitespace().collect();
                match commands[0] {
                    "def-block" => {
                        BlockComponent::Attribute(BlockAttribute::Name(commands[1].to_owned()))
                    }
                    _ => return Err(String::from("Invalid command")),
                }
            }
        };

        Ok(component)
    }
}

enum ParserState {
    Content,
    CommandFlag,
    Command,
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
            let content = ParsedElement::Content(builder.clone());
            builder.clear();
            elements.push(content);
            ParserState::Command
        }
        fn close_command(builder: &mut String, elements: &mut Vec<ParsedElement>) -> ParserState {
            let command = ParsedElement::Command(builder.clone());
            builder.clear();
            elements.push(command);
            ParserState::Content
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
            };
        }

        let components = elements.iter().filter_map(|e| e.try_into().ok()).collect();

        Block::validate(components)
    }
}
