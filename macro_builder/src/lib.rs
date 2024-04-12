

use proc_macro2::{TokenStream, Ident};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Attribute, DeriveInput, FieldsNamed, Meta, MetaList};

fn create_enum_field(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields.named.iter().map(|x| {
        let name = format!("{}", x.ident.as_ref().unwrap());
        let first_letter = name.get(..1).unwrap().to_ascii_uppercase();
        let field_name = format!("{}{}", first_letter, name.get(1..).unwrap());
        let name_idt = syn::Ident::new(&field_name, x.span());
        let inner = x.ty.clone();
        if x.attrs.len() > 0 {
            let type_name = format_ident!("{}FieldType", name_idt);
            return quote! {
                #name_idt(#type_name)
            }
            }
        quote! {
            #name_idt(#inner)
        }
    }).collect()
}

fn create_apply_matches(fields: &FieldsNamed, enum_idt: &Ident) -> Vec<TokenStream> {
    fields.named.iter().map(|x| {
        let member_name = format_ident!("{}", x.ident.as_ref().unwrap());

        let name = format!("{}", x.ident.as_ref().unwrap());
        let first_letter = name.get(..1).unwrap().to_ascii_uppercase();
        let field_name = format!("{}{}", first_letter, name.get(1..).unwrap());
        let name_idt = syn::Ident::new(&field_name, x.span());
        if x.attrs.len() > 0 {
            return quote! {
                #enum_idt::#name_idt(x) => self.#member_name.apply(x)
            }
            }
        quote! {
            #enum_idt::#name_idt(x) => self.#member_name = x
        }
    }).collect()
}

fn get_variance(attrs: &[Attribute]) -> Option<syn::LitInt> {
    for attr in attrs {
        if let Meta::List(MetaList{path, tokens, ..}) = &attr.meta {
            let identifier = path.segments.first();
            if identifier.is_none() {
                continue;
            }
            let identifier = identifier.unwrap();
            if identifier.ident.to_string() != "variance" {
                continue;
            }
            let result = syn::LitInt::new(&tokens.to_string(), attr.span());
            return Some(result)
        } else {
            continue
        };
    }
    None
}

#[proc_macro_derive(Builder, attributes(expand, variance))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    // eprintln!("ast: {ast:#?}");
    let name = ast.ident;

    let enum_type_idt = format_ident!("{name}FieldType");
    
    let fields = if let syn::Data::Struct(syn::DataStruct{ fields: syn::Fields::Named(ref named, ..), ..}) = ast.data {
        named
    } else {
        unimplemented!()
    };

    // Enum members
    let enum_fields = create_enum_field(fields);

    // apply match arguments
    let apply_matches = create_apply_matches(fields, &enum_type_idt);

    // variant size constant
    let contant_name = format!("{}_VARIANT_SIZE", name).to_ascii_uppercase();
    let size_const_name = syn::Ident::new(&contant_name, name.span());
    let constant_literal = get_variance(&ast.attrs).unwrap();
    
    // setters
    let setters_name = format_ident!("{}Setters", name);
    let setter_members = fields.named.iter().map(|x| {
        let member_name = format_ident!("{}", x.ident.as_ref().unwrap());
        //size
        let s = x.ty.clone();
        
        if x.attrs.len() > 0  {
            let setter_name = format!("{}Setters", s.to_token_stream().to_string());
            let setter_idt = syn::Ident::new(&setter_name, s.span());
            return quote! {
                fn #member_name(&mut self) -> impl #setter_idt;
            }
    
        }
        quote! {
            fn #member_name(&mut self) -> impl FieldSetter<#s>;
        }
    });
    let setter_accessors = fields.named
        .iter()
        .enumerate()
        .map(|(i,x)| {
        let member_name = format_ident!("{}", x.ident.as_ref().unwrap());
        let name = format!("{}", x.ident.as_ref().unwrap());
        let first_letter = name.get(..1).unwrap().to_ascii_uppercase();
        let field_name = format!("{}{}", first_letter, name.get(1..).unwrap());
        let enum_name_idt = syn::Ident::new(&field_name, x.span());
        //size
        let s = x.ty.clone();
        let index = syn::LitInt::new(&i.to_string(), x.span());
        if x.attrs.len() > 0  {
            let setter_name = format!("{}Setters", s.to_token_stream().to_string());
            let setter_idt = syn::Ident::new(&setter_name, s.span());

            // eprintln!("ee: {x:#?}");
            return quote! {
                fn #member_name(&mut self) -> impl #setter_idt {
                    let f = self.1;
                    SettersImpl(&mut self.0[1..(#constant_literal)], move |x| f(#enum_type_idt::#enum_name_idt(x)))
                }
            }
    
        }
        quote! {
            fn #member_name<'b>(&'b mut self) -> impl FieldSetter<#s> + 'b {
                let f = self.1;
                FieldSetterImpl::<_, T, #index, _>(self.0, move |x| f(#enum_type_idt::#enum_name_idt(x)), PhantomData)
            }
        }
    });

    // observer
    let observer_name = format_ident!("{}FieldObserver", name);
    

    quote! {
        #[derive(Debug, PartialEq, Eq)]
        pub enum #enum_type_idt  {
            #(#enum_fields),*
        }

        impl #name {
            pub fn apply(&mut self, field: #enum_type_idt) {
                match field {
                    #(#apply_matches),*
                }
            }
        }

        pub const #size_const_name: usize = #constant_literal;

        pub trait #setters_name {
            #(#setter_members)*
        }

        impl<'a, T, F: Fn(#enum_type_idt) -> T + Copy> #setters_name for SettersImpl<'a, T, F> {
            #(#setter_accessors)*
        }

        pub struct #observer_name([Option<#enum_type_idt>; #size_const_name]);

        impl #observer_name {
            pub fn new() -> Self {
                Self(Default::default())
            }

            pub fn setters<'a>(&'a mut self) -> impl #setters_name + 'a {
                SettersImpl(&mut self.0, Some)
            }

            pub fn events(&self) -> impl Iterator<Item = &#enum_type_idt> {
                self.0.iter().flatten()
            }

            pub fn clear_events(&mut self) {
                self.0.fill_with(|| None)
            }       
        }
    }.into()
}