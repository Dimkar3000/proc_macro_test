

use proc_macro2::{TokenStream, Ident};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Attribute, DeriveInput, FieldsNamed, Meta, MetaList};

fn get_enum_fields(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields.named.iter().map(|x| {
        let name = format!("{}", x.ident.as_ref().unwrap());
        let first_letter = name.get(..1).unwrap().to_ascii_uppercase();
        let field_name = format!("{}{}", first_letter, name.get(1..).unwrap());
        let name_idt = Ident::new(&field_name, x.span());
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

fn get_apply_match_cases(fields: &FieldsNamed, enum_idt: &Ident) -> Vec<TokenStream> {
    fields.named.iter().map(|x| {
        let member_name = format_ident!("{}", x.ident.as_ref().unwrap());

        let name = format!("{}", x.ident.as_ref().unwrap());
        let first_letter = name.get(..1).unwrap().to_ascii_uppercase();
        let field_name = format!("{}{}", first_letter, name.get(1..).unwrap());
        let name_idt = Ident::new(&field_name, x.span());
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
fn create_enum(enum_type_idt: &Ident, fields: &FieldsNamed) -> TokenStream {
    let enum_fields = get_enum_fields(fields);
    quote! {
        #[derive(Debug, PartialEq, Eq)]
        pub enum #enum_type_idt  {
            #(#enum_fields),*
        }
    }
}
fn create_setters(setters_name: &Ident, fields: &FieldsNamed, variance: &syn::LitInt, enum_idt: &Ident) -> TokenStream {
    let setter_members = fields.named.iter().map(|x| {
        let member_name = format_ident!("{}", x.ident.as_ref().unwrap());
        //size
        let s = x.ty.clone();
        
        if x.attrs.len() > 0  {
            let setter_name = format!("{}Setters", s.to_token_stream().to_string());
            let setter_idt = Ident::new(&setter_name, s.span());
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
        let enum_name_idt = Ident::new(&field_name, x.span());
        //size
        let s = x.ty.clone();
        let index = syn::LitInt::new(&i.to_string(), x.span());
        if x.attrs.len() > 0  {
            let setter_name = format!("{}Setters", s.to_token_stream().to_string());
            let setter_idt = Ident::new(&setter_name, s.span());

            // eprintln!("ee: {x:#?}");
            return quote! {
                fn #member_name(&mut self) -> impl #setter_idt {
                    let f = self.1;
                    SettersImpl(&mut self.0[1..(#variance)], move |x| f(#enum_idt::#enum_name_idt(x)))
                }
            }
    
        }
        quote! {
            fn #member_name<'b>(&'b mut self) -> impl FieldSetter<#s> + 'b {
                let f = self.1;
                FieldSetterImpl::<_, T, #index, _>(self.0, move |x| f(#enum_idt::#enum_name_idt(x)), PhantomData)
            }
        }
    });
    quote! {
        pub trait #setters_name {
            #(#setter_members)*
        }

        impl<'a, T, F: Fn(#enum_idt) -> T + Copy> #setters_name for SettersImpl<'a, T, F> {
            #(#setter_accessors)*
        }
    }
}

fn create_observer(name: &Ident, enum_idt: &Ident,variance: &syn::LitInt) -> TokenStream {
    let observer_name = format_ident!("{}FieldObserver", name);
    let setters_name = format_ident!("{}Setters", name);
    quote! {
        pub struct #observer_name([Option<#enum_idt>; #variance]);

        impl #observer_name {
            pub fn new() -> Self {
                Self(Default::default())
            }

            pub fn setters<'a>(&'a mut self) -> impl #setters_name + 'a {
                SettersImpl(&mut self.0, Some)
            }

            pub fn events(&self) -> impl Iterator<Item = &#enum_idt> {
                self.0.iter().flatten()
            }

            pub fn clear_events(&mut self) {
                self.0.fill_with(|| None)
            }       
        }
    }
}

fn create_type_imp_block(name: &Ident, fields: &FieldsNamed, enum_idt: &Ident) -> TokenStream {
    let apply_matches = get_apply_match_cases(fields, &enum_idt);
    quote! {
        impl #name {
            pub fn apply(&mut self, field: #enum_idt) {
                match field {
                    #(#apply_matches),*
                }
            }
        }
    }
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
    let enum_type = create_enum(&enum_type_idt, &fields);

    // apply match arguments
    let type_impl_block = create_type_imp_block(&name,fields, &enum_type_idt);

    // variant size constant
    let contant_name = format!("{}_VARIANT_SIZE", name).to_ascii_uppercase();
    let size_const_name = Ident::new(&contant_name, name.span());
    let constant_literal = get_variance(&ast.attrs).unwrap();
    
    // setters
    let setters_name = format_ident!("{}Setters", name);
    let setters = create_setters(&setters_name,fields,&constant_literal,&enum_type_idt);
    
    // observer
    let observer = create_observer(&name,&enum_type_idt, &constant_literal);
    

    quote! {
        #enum_type

        #type_impl_block

        pub const #size_const_name: usize = #constant_literal;

        #setters

        #observer
    }.into()
}