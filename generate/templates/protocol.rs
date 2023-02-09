use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional,
    session,
    Branch,
    End,
    Message,
    Receive,
    Role,
    Roles,
    Select,
    Send,
    effect::{
        SideEffect,
        Constant,
        Incr,
    },
    try_session,
    predicate::*,
    ParamName,
    Param,
};

use std::collections::HashMap;
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

type Name = char;
type Value = i32;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
{%- for role in roles %}
    {{ role.snake }}: {{ role.camel }},
{%- endfor %}
}
{% for role in roles %}
#[derive(Role)]
#[message(Label)]
struct {{ role.camel }} {
{%- for index in role.routes.iter() %}
    {%- let route = roles[index.0] %}
    #[route({{ route.camel }})]
    {{ route.snake }}: Channel,
{%- endfor %}
}
{% endfor %}
#[derive(Message, Copy, Clone)]
enum Label {
{%- for label in labels %}
    {{ label.camel }}({{ label.camel }}),
{%- endfor %}
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
        {%- for label in labels %}
            Label::{{ label.camel }}(payload) => payload.into(),
        {%- endfor %}
        }
    }
}

{% for label in labels %}
#[derive(Copy, Clone)]
struct {{ label.camel }}{% if !label.parameters.is_empty() -%}
    ({{ label.parameters|join(", ") }})
{%- endif %};

impl From<{{ label.camel }}> for Value {
    fn from(value: {{ label.camel }}) -> Value {
        let {{ label.camel }}(val) = value;
        val
    }
}
{% endfor %}
{%- for role in roles %}
{%- for (i, definition) in role.definitions.iter().rev().enumerate() %}
{%- let node = role.nodes[definition.node] %}
#[session(Name, Value)]
{%- match definition.body %}
{%- when DefinitionBody::Type with { safe, ty } %}
{%- if safe|copy_bool %}
type {{ camel }}{{ role.camel }}{% if i > 0 -%}{{ node }}{%- endif %} = {{ ty|ty(camel, role, roles, labels) }};
{%- else %}
struct {{ camel }}{{ role.camel }}{% if i > 0 -%}{{ node }}{%- endif %}({{ ty|ty(camel, role, roles, labels) }});
{%- endif %}
{%- when DefinitionBody::Choice with (choices) %}
enum {{ camel }}{{ role.camel }}{{ node }} {
{%- for choice in choices %}
    {%- let label = labels[choice.label] %}
    {{ label.camel }}({{ label.camel }}, {{ choice.ty|ty(camel, role, roles, labels) }}),
{%- endfor %}
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for {{ camel }}{{ role.camel }}{{ node }}<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            {%- for choice in choices %}
            {%- let label = labels[choice.label] %}
            Label::{{ label.camel }}({{ label.camel }}(val)) => {
                    ('{{ label.param_names[0] }}', *val)
            }
            {%- endfor %}
            _ => panic!("Unexpected label"),
        }
    }
}

{%- for choice in choices %}
{%- let label = labels[choice.label] %}
impl<'__r, __R: ::rumpsteak::Role> ParamName<{{ label.camel }}, Name> for {{ camel }}{{ role.camel }}{{ node }}<'__r, __R> {
    fn get_param_name() -> Name {
        '{{ label.param_names[0] }}'
    }
}
{%- endfor %}

#[derive(Default)]
struct {{ camel }}{{ role.camel }}{{ node }}Predicate {}
impl Predicate for {{ camel }}{{ role.camel }}{{ node }}Predicate {
    type Name = Name;
    type Value = Value;
    type Label = Label;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        label: Option<&Self::Label>
    ) -> Result<(), Self::Error> {
        if let Some(label) = label {
            match label {
                {%- for choice in choices %}
                {%- let label = labels[choice.label] %}
                Label::{{ label.camel }}(_) => {
                    {{ choice.predicate }}::default()
                        .check(m, Some(label))
                    },
                {%- endfor %}
                _ => {
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }
}
{%- endmatch %}
{% endfor %}
{%- endfor %}
