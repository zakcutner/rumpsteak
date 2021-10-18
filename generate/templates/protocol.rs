use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send,
};

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;
type Name = {{ name_str }};
type Value = {{ value_str }};

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
#[derive(Message)]
enum Label {
{%- for label in labels %}
    {{ label.camel }}({{ label.camel }}),
{%- endfor %}
}
{% for label in labels %}
struct {{ label.camel }}{% if !label.parameters.is_empty() -%}
    ({{ label.parameters|join(", ") }})
{%- endif %};
{% endfor %}
{%- for role in roles %}
{%- for (i, definition) in role.definitions.iter().rev().enumerate() %}
{%- let node = role.nodes[definition.node] %}
#[session]
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
{%- endmatch %}
{% endfor %}
{%- endfor %}
