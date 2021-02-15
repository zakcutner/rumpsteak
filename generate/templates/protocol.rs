#[allow(unused_imports)]
use ::rumpsteak::{
    choice::Choice,
    role::{Role, Roles, ToFrom},
    Branch, End, IntoSession, Label, Receive, Select, Send,
};

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
{%- for role in roles %}
    {{ role.snake }}: {{ role.camel }},
{%- endfor %}
}
{% for role in roles %}
#[derive(Role)]
#[message(Message)]
struct {{ role.camel }} {
{%- for index in role.routes.iter() %}
    {%- let route = roles[index.0] %}
    #[route({{ route.camel }})]
    {{ route.snake }}: ToFrom<{{ route.camel }}>,
{%- endfor %}
}
{% endfor %}
#[derive(Label)]
enum Message {
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
{%- for definition in role.definitions.iter().rev() %}
{%- match definition %}
{%- when Definition::Type with { safe, index, ty } %}
{%- if safe|copy_bool %}
type {{ camel }}{{ role.camel }}{{ index }}<'r> = {{ ty|ty(camel, role.camel, roles, labels) }};
{%- else %}
#[derive(IntoSession)]
#[role('r, {{ role.camel }})]
struct {{ camel }}{{ role.camel }}{{ index }}<'r>({{ ty|ty(camel, role.camel, roles, labels) }});
{%- endif %}
{%- when Definition::Choice with { index, choices } %}
#[derive(Choice)]
#[role('r, {{ role.camel }})]
enum {{ camel }}{{ role.camel }}{{ index }}<'r> {
{%- for choice in choices %}
    {%- let label = labels[choice.label] %}
    {{ label.camel }}({{ label.camel }}, {{ choice.ty|ty(camel, role.camel, roles, labels) }}),
{%- endfor %}
}
{%- endmatch %}
{% endfor %}
{%- endfor %}
