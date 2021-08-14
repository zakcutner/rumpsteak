use quote::{format_ident, quote};
use std::{env, fs::File, io::Write, path::PathBuf};

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut file = File::create(out_path.join("rumpsteak.rs")).unwrap();

    for n in [5usize] {
        let ident_upper = format_ident!("SourceOptimized{}", n);
        let ident_lower_source = format_ident!("source_optimized_{}", n);
        let ident_lower_run = format_ident!("run_optimized_{}", n);

        let mut ty = quote!(Source);

        for _ in 0..n {
            ty = quote!(Receive<T, Ready, #ty>);
        }

        for _ in 0..n {
            ty = quote!(Send<T, Value, #ty>);
        }

        let sends = (0..n).map(|_| quote!(let s = s.send(Value(*values.next().unwrap())).await?;));
        let receives = (0..n).map(|_| quote!(let (Ready, s) = s.receive().await?;));

        let output = quote! {
            #[session]
            type #ident_upper = #ty;

            async fn #ident_lower_source(role: &mut S, values: &[i32]) -> Result<()> {
                try_session(role, |s: #ident_upper<'_, _>| async {
                    let mut values = values.iter();
                    #(#sends)*
                    #(#receives)*
                    source_inner(s, values).await
                })
                .await
            }

            pub async fn #ident_lower_run(input: Arc<[i32]>) {
                let Roles(mut s, mut t) = Roles::default();
                let (_, output) = try_join!(
                    {
                        let input = input.clone();
                        tokio::spawn(async move {  #ident_lower_source(&mut s, &input).await.unwrap() })
                    },
                    tokio::spawn(async move { sink(&mut t).await.unwrap() }),
                )
                .unwrap();
                assert_eq!(input.as_ref(), output.as_slice());
            }
        };

        write!(file, "{}", output).unwrap();
    }
}
