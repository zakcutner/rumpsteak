use crepe::crepe;

use dot_parser::canonical::Graph;
use dot_parser::*;
use std::env;
use std::env::current_dir;
use std::fs;
mod parser;
use parser::Label;

use std::str::FromStr;

#[macro_use]
extern crate pest_derive;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct Participant {
    name: char,
}

impl std::fmt::Display for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.name)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct State {
    index: i32,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Γ_{}", self.index)
    }
}

crepe! {
    @input
    struct Send(State, Participant, char, Participant, State);

    // To go from s1 to s2, we must verify a refinement that contains the current free variable
    @input
    struct FreeVariableRefinement(State, char, State);

    @output
    struct In(State, Participant, char);

    @output
    struct NotVerifiableFV(State, State, char, char, Participant, Participant);

    @output
    struct NotVerifiableDup(State, char, Participant, Participant);

    // Immediate deductions based on send
    In(s1, p1, var) <- Send(s1, p1, var, _, _);
    In(s2, p2, var) <- Send(_, _, var, p2, s2);

    // Backward deductions (what can we infer regarding s1 from s2)
    In(s1, p1, var1) <- In(s2, p1, var1),
        Send(s1, p1, _, _, s2);
    In(s1, p2, var1) <- In(s2, p2, var1),
        Send(s1, _, var2, p2, s2),
        (var1 != var2);

    // Forward deductions (what can we infer regarding s2 from s1)
    In(s2, p2, var1) <- In(s1, p2, var1),
        Send(s1, _, _, p2, s2);
    In(s2, p1, var1) <- In(s1, p1, var1),
        Send(s1, p1, var2, _, s2),
        (var1 != var2);

    // Verifiability
    NotVerifiableFV(s1, s2, var1, var2, p1, p2) <- FreeVariableRefinement(s1, var1, s2),
        FreeVariableRefinement(s1, var2, s2),
        In(s1, p1, var1),
        In(s1, p2, var2),
        (p1.name != p2.name);

    NotVerifiableDup(s, var, p1, p2) <- In(s, p1, var),
        In(s, p2, var),
        (p1.name != p2.name);
}

fn filter<'a>(a: (&'a str, &'a str)) -> Option<Label> {
    let (name, value) = a;
    if name == "label" {
        Label::from_str(value).ok()
    } else {
        None
    }
}

fn generate(graph: Graph<Label>) -> (Vec<Send>, Vec<FreeVariableRefinement>) {
    let mut list = Vec::new();
    let mut fv = Vec::new();
    for edge in graph.edges.set {
        if edge.attr.elems.len() > 0 {
            let label = edge.attr.elems[0].clone();
            for (param, _) in label.parameters {
                eprintln!(
                    "
                Send(
                    State {{
                        index: {},
                    }},
                    Participant {{ name: {} }},
                    {},
                    Participant {{
                        name: {},
                    }},
                    State {{
                        index: {},
                    }},
                )
                ",
                    edge.from, label.sender, param, label.receiver, edge.to
                );
                list.push(Send(
                    State {
                        index: FromStr::from_str(edge.from).unwrap(),
                    },
                    Participant { name: label.sender },
                    param,
                    Participant {
                        name: label.receiver,
                    },
                    State {
                        index: FromStr::from_str(edge.to).unwrap(),
                    },
                ))
            }
            for refinement in label.refinements {
                fv.push(FreeVariableRefinement(
                    State {
                        index: FromStr::from_str(edge.from).unwrap(),
                    },
                    refinement,
                    State {
                        index: FromStr::from_str(edge.to).unwrap(),
                    },
                ))
            }
        }
    }

    return (list, fv);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprint!("Please use the command 'cargo run xxx'");
        return;
    };

    let mut file = String::from("protocols/");
    file += &args[1];
    file += ".txt";

    let dir = current_dir().unwrap().join(file.as_str());
    let filepath = dir.to_str().unwrap();
    let file = fs::read_to_string(filepath);
    if let Err(e) = &file {
        eprint!("{:#?}", e);
    }
    let raw = &file.unwrap();
    let graph = ast::Graph::read_dot(raw).unwrap();
    let canonical_graph: canonical::Graph<'_, _> = graph.into();
    let filtered_graph = canonical_graph.filter_map(|a| filter(a));

    let mut runtime = Crepe::new();
    let (transitions, fv) = generate(filtered_graph);
    runtime.extend(&transitions);
    runtime.extend(&fv);

    for FreeVariableRefinement(s1, param, s2) in fv {
        println!("FreeVariableRefinement {} {} {}", s1, param, s2);
    }

    let (set_in, errorfv, errordup) = runtime.run();
    for In(s, p, v) in set_in {
        println!("In state {}, variable {} is at location {}", s, v, p);
    }

    for NotVerifiableFV(s1, s2, v1, v2, p1, p2) in errorfv {
        println!("[ERROR] Can not verify refinement from state {} to state {}, variable {} is at {} while variable {} is at {}", s1, s2, v1, p1, v2, p2);
    }

    for NotVerifiableDup(s, v, p1, p2) in errordup {
        println!(
            "[ERROR] In state {}, variable {} is both at {} and {}",
            s, v, p1, p2
        );
    }
}
