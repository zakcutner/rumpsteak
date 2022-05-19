extern crate pest;

use pest::iterators::Pair;
use pest::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]

struct DigraphParser;

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
pub(in crate) enum Direction {
    Send,
    Receive,
}

impl<'a> TryFrom<Rule> for Direction {
    type Error = ();
    fn try_from(value: Rule) -> Result<Self, Self::Error> {
        match value {
            Rule::send => Ok(Direction::Send),
            Rule::receive => Ok(Direction::Receive),
            _ => Err(()),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
pub struct Label<'a> {
    pub(in crate) sender: char,                     // "A"
    pub(in crate) direction: Direction,             // Send
    pub(in crate) receiver: char,                   // "C"
    pub(in crate) payload: &'a str,                 // "empty3"
    pub(in crate) parameters: Vec<(char, &'a str)>, // (name, type) ("x", "u32")
    pub(in crate) refinements: Vec<char>, // ['x']

}

impl<'a> Label<'a> {
    pub(in crate) fn parse(p: Pair<'a, Rule>) -> Result<Self, ()> {
        if let Rule::label = p.as_rule() {
            let mut inner = p.into_inner();
            // let from = inner.next().unwrap().as_str();
            // let to = inner.next().unwrap().as_str();

            let sender_str = inner.next().unwrap().as_str();
            assert_eq!(sender_str.len(), 1);
            let sender = sender_str.chars().nth(0).unwrap();
            let direction = inner.next().unwrap().as_rule().try_into().unwrap();
            let receiver_str = inner.next().unwrap().as_str();
            assert_eq!(receiver_str.len(), 1);
            let receiver = receiver_str.chars().nth(0).unwrap();
            let payload = inner.next().unwrap().as_str();
            let mut parameters = Vec::new();
            let params = inner.next();
            // eprintln!("{:#?}", params);
            for pair in params {
                if pair.as_str() != "" {
                    let inner = pair.into_inner();
                    for param in inner {
                        let mut inner = param.into_inner();
                        let name_str = inner.next().unwrap().as_str();
                        assert_eq!(name_str.len(), 1);
                        let name = name_str.chars().nth(0).unwrap();
                        let typ = inner.next().unwrap().as_str();
                        parameters.push((name, typ));
                    }
                }
            }
            // let predicate = inner.next().map(|p| p.as_str());
            let mut refinements = Vec::new();
            while let Some(p) = inner.next() {
                match p.as_rule() {
                    // case 1: x < 10 
                    // case 2: x < y
                    Rule::predicate => {
                        let mut inner = p.clone().into_inner();
                        // the first operand could only be variable
                        let var_str = inner.next().unwrap().as_str();
                        assert_eq!(var_str.len(), 1);
                        let var = var_str.chars().nth(0).unwrap();
                        refinements.push(var);
                        // the first operand could be either value or variable
                        let param_str = inner.next().unwrap().as_str();
                        let param = param_str.chars().nth(0).unwrap();
                        if !param.is_numeric() {
                            refinements.push(param);
                        }
                    }
                    Rule::side_effect => {
                        let mut inner = p.clone().into_inner();
                        let param1_str = inner.next().unwrap().as_str();
                        assert_eq!(param1_str.len(), 1);
                        let param1 = param1_str.chars().nth(0).unwrap();
                        let param2_str = inner.next().unwrap().as_str();
                        assert_eq!(param2_str.len(), 1);
                        let param2 = param2_str.chars().nth(0).unwrap();

                        assert_eq!(param1, param2);
                        // let op = inner.next().unwrap();
                        let value = inner.next().unwrap().as_str();
                        refinements.push(param1);
                    }
                    _ => (),
                }
            }
            // eprintln!("{:#?}, {:#?}, {:#?}, {:#?}, {:#?}", role, direction, payload, parameters, predicate);
            Ok(Label {
                sender,
                direction,
                receiver,
                payload,
                parameters,
                refinements,
            })
        } else {
            Err(())
        }
    }
    pub(in crate) fn from_str(s: &'a str) -> Result<Self, ()> {
        let label = DigraphParser::parse(Rule::label, s);
        if let Err(e) = &label {
            println!("{}", e);
        }
        Label::parse(label.unwrap().next().unwrap())
    }
}
