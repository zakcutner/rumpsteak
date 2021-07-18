use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

struct Mesh {
    roles: usize,
    roles_upper: Vec<Ident>,
    roles_lower: Vec<Ident>,
}

impl Mesh {
    fn new(roles: usize) -> Self {
        Self {
            roles,
            roles_upper: (0..roles).map(|role| format_ident!("R{}", role)).collect(),
            roles_lower: (0..roles).map(|role| format_ident!("r{}", role)).collect(),
        }
    }

    fn rumpsteak(&self) -> TokenStream {
        let roles_upper = &self.roles_upper;
        let roles_lower = &self.roles_lower;

        let roles = roles_upper.iter().enumerate().map(|(i, role)| {
            let others = roles_upper
                .iter()
                .enumerate()
                .filter_map(|(j, role)| (i != j).then(|| role));

            quote! {
                #[derive(Role)]
                #[message(Hello)]
                struct #role(#(#[route(#others)] Channel),*);
            }
        });

        let sessions =
            roles_upper
                .iter()
                .zip(roles_lower)
                .enumerate()
                .map(|(i, (role_upper, role_lower))| {
                    let (before, after) = (&roles_upper[..i], &roles_upper[i + 1..]);
                    let (mut left, mut right) = (TokenStream::new(), TokenStream::new());
                    let mut statements = TokenStream::new();

                    for other in before {
                        left.extend(quote!(Receive<#other, Hello, Send<#other, Hello, ));
                        right.extend(quote!(>>));

                        statements.extend(quote! {
                            let (Hello, s) = s.receive().await?;
                            let s = s.send(Hello).await?;
                        });
                    }

                    for other in after {
                        left.extend(quote!(Send<#other, Hello, Receive<#other, Hello, ));
                        right.extend(quote!(>>));

                        statements.extend(quote! {
                            let s = s.send(Hello).await?;
                            let (Hello, s) = s.receive().await?;
                        });
                    }

                    let ident_upper = format_ident!("Mesh{}", role_upper);
                    let ident_lower = format_ident!("mesh_{}", role_lower);

                    quote! {
                        #[session]
                        type #ident_upper = #left End #right;

                        async fn #ident_lower(role: &mut #role_upper) -> Result<()> {
                            try_session(role, |s: #ident_upper<'_, _>| async {
                                #statements
                                Ok(((), s))
                            })
                            .await
                        }
                    }
                });

        let idents_lower = roles_lower
            .iter()
            .map(|role| format_ident!("mesh_{}", role));

        quote! {
            use futures::{
                channel::mpsc::{UnboundedReceiver, UnboundedSender},
                try_join,
            };
            use rumpsteak::{
                channel::Bidirectional, session, try_session, End, Message, Receive, Role, Roles, Send,
            };
            use std::{error::Error, result};

            type Result<T> = result::Result<T, Box<dyn Error>>;

            type Channel = Bidirectional<UnboundedSender<Hello>, UnboundedReceiver<Hello>>;

            #[derive(Roles)]
            struct Roles(#(#roles_upper),*);

            #(#roles)*

            #[derive(Message)]
            struct Hello;

            #(#sessions)*

            pub async fn run() {
                let Roles(#(mut #roles_lower),*) = Roles::default();
                try_join!(#(#idents_lower(&mut #roles_lower)),*).unwrap();
            }
        }
    }

    fn mpstthree(&self) -> TokenStream {
        let roles = self.roles;
        let roles_upper = &self.roles_upper;
        let roles_lower = &self.roles_lower;

        let normal_roles = roles_upper.iter().map(|role| {
            let dual = format_ident!("{}Dual", role);
            quote!(#role, #dual |)
        });

        let session_bundles = roles_upper.iter().zip(roles_lower).enumerate().map(
            |(i, (role_upper_left, role_lower_left))| {
                let session_bundles = roles_upper.iter().zip(roles_lower).enumerate().filter_map(
                    |(j, (role_upper_right, role_lower_right))| {
                        if i == j {
                            return None;
                        }

                        let send_ident =
                            format_ident!("send_mpst_{}_to_{}", role_lower_left, role_lower_right);
                        let receive_ident = format_ident!(
                            "recv_mpst_{}_from_{}",
                            role_lower_left,
                            role_lower_right
                        );

                        let send_index = j + 1;
                        let receive_index = i + 1;

                        Some(quote! {
                            create_send_mpst_session_bundle!(
                                #send_ident, #role_upper_right, #send_index | =>
                                #role_upper_left, MeshedChannels, #roles
                            );

                            create_recv_mpst_session_bundle!(
                                #receive_ident, #role_upper_right, #receive_index | =>
                                #role_upper_left, MeshedChannels, #roles
                            );
                        })
                    },
                );

                session_bundles.collect::<TokenStream>()
            },
        );

        quote! {
            #![allow(clippy::nonstandard_macro_braces)]

            use mpstthree::{
                binary::struct_trait::{End, Recv, Send, Session},
                bundle_struct_fork_close_multi, choose_mpst_multi_to_all, create_multiple_normal_role,
                create_recv_mpst_session_bundle, create_send_mpst_session_bundle, offer_mpst,
                role::{broadcast::RoleBroadcast, end::RoleEnd},
            };
            use std::{error::Error, result};

            type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

            bundle_struct_fork_close_multi!(close_mpst, fork_mpst, MeshedChannels, #roles);

            create_multiple_normal_role!(#(#normal_roles)*);

            #(#session_bundles)*

            struct Hello;


        }
    }
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::create_dir_all(out_path.join("rumpsteak")).unwrap();
    fs::create_dir_all(out_path.join("mpstthree")).unwrap();

    for roles in (2..=8).filter(|roles| roles % 2 == 0) {
        let mesh = Mesh::new(roles);

        let mut file = File::create(out_path.join(format!("rumpsteak/{}.rs", roles))).unwrap();
        write!(file, "{}", mesh.rumpsteak()).unwrap();

        let mut file = File::create(out_path.join(format!("mpstthree/{}.rs", roles))).unwrap();
        write!(file, "{}", mesh.mpstthree()).unwrap();
    }
}
