use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Field, Fields, Ident, parse_macro_input};

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

/// Derive macro for `renderer::AsVertexLayout`.
///
/// Container attributes currently supported:
/// - `#[vertex(step_mode = Vertex)]`
/// - `#[vertex(step_mode = Instance)]`
///
/// Field attributes currently supported:
/// - `#[vertex_attribute(location = <u32>, format = <VertexFormat>)]`
#[proc_macro_derive(AsVertexLayout, attributes(vertex, vertex_attribute))]
pub fn derive_as_vertex_layout(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    let parsed = match parse_vertex_layout_struct(&input) {
        Ok(parsed) => parsed,
        Err(error) => return error.to_compile_error().into(),
    };

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let step_mode = parsed.step_mode.variant_ident();

    let attributes = parsed.attributes.iter().map(|attribute| {
        let field_ident = &attribute.field_ident;
        let location = attribute.location;
        let format = attribute.format.variant_ident();

        quote! {
            renderer::VertexAttribute {
                format: renderer::VertexFormat::#format,
                offset: core::mem::offset_of!(#ident #ty_generics, #field_ident) as u64,
                shader_location: #location,
            }
        }
    });

    quote! {
        impl #impl_generics renderer::AsVertexLayout for #ident #ty_generics #where_clause {
            fn vertex_buffer_layout() -> renderer::VertexBufferLayout {
                const ATTRIBUTES: &[renderer::VertexAttribute] = &[
                    #(#attributes),*
                ];

                renderer::VertexBufferLayout {
                    array_stride: core::mem::size_of::<Self>() as u64,
                    step_mode: renderer::VertexStepMode::#step_mode,
                    attributes: ATTRIBUTES,
                }
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

struct ParsedVertexLayoutStruct {
    step_mode: ParsedVertexStepMode,
    attributes: Vec<ParsedVertexAttribute>,
}

struct ParsedVertexAttribute {
    field_ident: Ident,
    location: u32,
    format: ParsedVertexFormat,
}

enum ParsedVertexStepMode {
    Vertex,
    Instance,
}

impl ParsedVertexStepMode {
    fn variant_ident(&self) -> Ident {
        match self {
            Self::Vertex => Ident::new("Vertex", proc_macro2::Span::call_site()),
            Self::Instance => Ident::new("Instance", proc_macro2::Span::call_site()),
        }
    }
}

enum ParsedVertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
}

impl ParsedVertexFormat {
    fn parse(format_ident: &Ident) -> syn::Result<Self> {
        match format_ident.to_string().as_str() {
            "Float32" => Ok(Self::Float32),
            "Float32x2" => Ok(Self::Float32x2),
            "Float32x3" => Ok(Self::Float32x3),
            "Float32x4" => Ok(Self::Float32x4),
            "Uint32" => Ok(Self::Uint32),
            "Uint32x2" => Ok(Self::Uint32x2),
            "Uint32x3" => Ok(Self::Uint32x3),
            "Uint32x4" => Ok(Self::Uint32x4),
            "Sint32" => Ok(Self::Sint32),
            "Sint32x2" => Ok(Self::Sint32x2),
            "Sint32x3" => Ok(Self::Sint32x3),
            "Sint32x4" => Ok(Self::Sint32x4),
            _ => Err(syn::Error::new_spanned(
                format_ident,
                "unsupported vertex format",
            )),
        }
    }

    fn variant_ident(&self) -> Ident {
        let name = match self {
            Self::Float32 => "Float32",
            Self::Float32x2 => "Float32x2",
            Self::Float32x3 => "Float32x3",
            Self::Float32x4 => "Float32x4",
            Self::Uint32 => "Uint32",
            Self::Uint32x2 => "Uint32x2",
            Self::Uint32x3 => "Uint32x3",
            Self::Uint32x4 => "Uint32x4",
            Self::Sint32 => "Sint32",
            Self::Sint32x2 => "Sint32x2",
            Self::Sint32x3 => "Sint32x3",
            Self::Sint32x4 => "Sint32x4",
        };

        Ident::new(name, proc_macro2::Span::call_site())
    }
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

/// Parse and validate the struct that `AsVertexLayout` is being derived for.
fn parse_vertex_layout_struct(input: &DeriveInput) -> syn::Result<ParsedVertexLayoutStruct> {
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(fields) => {
                return Err(syn::Error::new_spanned(
                    fields,
                    "AsVertexLayout currently supports only structs with named fields",
                ));
            }
            Fields::Unit => {
                return Err(syn::Error::new_spanned(
                    data.struct_token,
                    "AsVertexLayout requires at least one named field",
                ));
            }
        },
        Data::Enum(data) => {
            return Err(syn::Error::new_spanned(
                data.enum_token,
                "AsVertexLayout can only be derived for structs",
            ));
        }
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "AsVertexLayout can only be derived for structs",
            ));
        }
    };

    let step_mode = parse_vertex_step_mode(&input.attrs)?;

    let mut attributes = Vec::new();
    for field in fields.iter() {
        if let Some(attribute) = parse_vertex_attribute(field)? {
            attributes.push(attribute);
        }
    }

    attributes.sort_by_key(|attribute| attribute.location);

    for pair in attributes.windows(2) {
        if pair[0].location == pair[1].location {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "duplicate vertex location {} in AsVertexLayout derive",
                    pair[0].location
                ),
            ));
        }
    }

    Ok(ParsedVertexLayoutStruct {
        step_mode,
        attributes,
    })
}

/// Parse `#[vertex(step_mode = ...)]` from the target struct attributes.
fn parse_vertex_step_mode(attrs: &[syn::Attribute]) -> syn::Result<ParsedVertexStepMode> {
    let mut step_mode = ParsedVertexStepMode::Vertex;

    for attr in attrs {
        if !attr.path().is_ident("vertex") {
            continue;
        }

        let mut local_step_mode: Option<ParsedVertexStepMode> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("step_mode") {
                let step_mode_ident = parse_meta_value_ident(&meta)?;
                let parsed = parse_vertex_step_mode_ident(&step_mode_ident)?;

                if local_step_mode.is_some() {
                    return Err(meta.error("duplicate step_mode in #[vertex(...)]"));
                }

                local_step_mode = Some(parsed);
                Ok(())
            } else {
                Err(meta.error("unsupported vertex container option"))
            }
        })?;

        if let Some(parsed) = local_step_mode {
            step_mode = parsed;
        }
    }

    Ok(step_mode)
}

/// Parse `#[vertex_attribute(location = <u32>, format = <VertexFormat>)]` on a field.
fn parse_vertex_attribute(field: &Field) -> syn::Result<Option<ParsedVertexAttribute>> {
    let mut parsed: Option<ParsedVertexAttribute> = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("vertex_attribute") {
            continue;
        }

        if parsed.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "duplicate #[vertex_attribute(...)] on field",
            ));
        }

        let field_ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(field.span(), "AsVertexLayout expects named struct fields")
        })?;

        let mut location: Option<u32> = None;
        let mut format: Option<ParsedVertexFormat> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("location") {
                if location.is_some() {
                    return Err(meta.error("duplicate location in #[vertex_attribute(...)]"));
                }

                let lit = meta.value()?.parse::<syn::LitInt>()?;
                location = Some(lit.base10_parse::<u32>()?);
                Ok(())
            } else if meta.path.is_ident("format") {
                if format.is_some() {
                    return Err(meta.error("duplicate format in #[vertex_attribute(...)]"));
                }

                let format_ident = parse_meta_value_ident(&meta)?;
                format = Some(ParsedVertexFormat::parse(&format_ident)?);
                Ok(())
            } else {
                Err(meta.error("unsupported vertex_attribute option"))
            }
        })?;

        let location = location.ok_or_else(|| {
            syn::Error::new_spanned(attr, "missing location in #[vertex_attribute(...)]")
        })?;

        let format = format.ok_or_else(|| {
            syn::Error::new_spanned(attr, "missing format in #[vertex_attribute(...)]")
        })?;

        parsed = Some(ParsedVertexAttribute {
            field_ident,
            location,
            format,
        });
    }

    Ok(parsed)
}

/// Parse an identifier value from an attribute item like `key = Value`.
fn parse_meta_value_ident(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<Ident> {
    let value = meta.value()?;
    let path = value.parse::<syn::Path>()?;

    if path.leading_colon.is_some() || path.segments.len() != 1 {
        return Err(meta.error("expected an identifier value"));
    }

    let segment = path
        .segments
        .first()
        .ok_or_else(|| meta.error("expected an identifier value"))?;

    Ok(segment.ident.clone())
}

/// Parse supported step mode values.
fn parse_vertex_step_mode_ident(step_mode: &Ident) -> syn::Result<ParsedVertexStepMode> {
    match step_mode.to_string().as_str() {
        "Vertex" => Ok(ParsedVertexStepMode::Vertex),
        "Instance" => Ok(ParsedVertexStepMode::Instance),
        _ => Err(syn::Error::new_spanned(
            step_mode,
            "unsupported step mode, expected Vertex or Instance",
        )),
    }
}
