use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, parse_macro_input};

/// Derive macro for `renderer::AsBindGroup`.
///
/// Field attributes currently supported:
/// - `#[uniform(<binding_index>)]`
#[proc_macro_derive(AsBindGroup, attributes(uniform))]
pub fn derive_as_bind_group(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    let parsed = match parse_bind_group_struct(&input) {
        Ok(parsed) => parsed,
        Err(error) => return error.to_compile_error().into(),
    };

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let layout_entries = parsed.fields.iter().map(|field| {
        let binding = field.binding;

        match field.kind {
            ParsedFieldKind::Uniform => quote! {
                renderer::BindGroupLayoutEntry {
                    binding: #binding,
                    visibility: renderer::ShaderStages::VertexFragment,
                    ty: renderer::BindingType::UniformBuffer,
                }
            },
        }
    });

    quote! {
        impl #impl_generics renderer::AsBindGroup for #ident #ty_generics #where_clause {
            fn layout_entries() -> &'static [renderer::BindGroupLayoutEntry] {
                const ENTRIES: &[renderer::BindGroupLayoutEntry] = &[
                    #(#layout_entries),*
                ];

                ENTRIES
            }
        }
    }
    .into()
}

/// Parsed representation of the target struct.
struct ParsedBindGroupStruct {
    fields: Vec<ParsedField>,
}

struct ParsedField {
    binding: u32,
    kind: ParsedFieldKind,
}

enum ParsedFieldKind {
    Uniform,
}

/// Parse and validate the struct that `AsBindGroup` is being derived for.
fn parse_bind_group_struct(input: &DeriveInput) -> syn::Result<ParsedBindGroupStruct> {
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        Data::Enum(data) => {
            return Err(syn::Error::new_spanned(
                data.enum_token,
                "AsBindGroup can only be derived for structs",
            ));
        }
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "AsBindGroup can only be derived for structs",
            ));
        }
    };

    let iter: Box<dyn Iterator<Item = &Field>> = match fields {
        Fields::Named(named) => Box::new(named.named.iter()),
        Fields::Unnamed(unnamed) => Box::new(unnamed.unnamed.iter()),
        Fields::Unit => Box::new(std::iter::empty()),
    };

    let mut parsed_fields = Vec::new();

    for field in iter {
        parsed_fields.push(parse_field(field)?);
    }

    parsed_fields.sort_by_key(|field| field.binding);

    for pair in parsed_fields.windows(2) {
        if pair[0].binding == pair[1].binding {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "duplicate binding {} in AsBindGroup derive",
                    pair[0].binding
                ),
            ));
        }
    }

    Ok(ParsedBindGroupStruct {
        fields: parsed_fields,
    })
}

/// Parse a single field annotation into bind group metadata.
fn parse_field(field: &Field) -> syn::Result<ParsedField> {
    let mut parsed: Option<ParsedField> = None;

    for attr in &field.attrs {
        if attr.path().is_ident("uniform") {
            if parsed.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "duplicate binding attribute on field",
                ));
            }

            let binding = attr.parse_args::<syn::LitInt>()?.base10_parse::<u32>()?;

            parsed = Some(ParsedField {
                binding,
                kind: ParsedFieldKind::Uniform,
            });
        }
    }

    parsed.ok_or_else(|| {
        syn::Error::new_spanned(
            field,
            "missing binding attribute: expected #[uniform(<binding_index>)]",
        )
    })
}
