#![allow(dead_code)]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TStream;
use syn::{
    Item, TraitItem, parse_macro_input, ItemTrait, ItemStruct,
    punctuated::Punctuated, FnArg, token::Comma, Pat,
};
use quote::quote;
use std::iter::FromIterator;

#[proc_macro_attribute]
pub fn interface (attr : TokenStream, item : TokenStream) -> TokenStream {
    let cloned = item.clone();
    let parsed : Item = parse_macro_input!(item as Item);

    match parsed {
        Item::Trait(trait_) => trait_interface(&trait_),
        Item::Struct(struct_) => struct_interface(&attr, &struct_),
        _ => cloned
    }
}

// generates a vtable for a trait
fn trait_interface (trait_ : &ItemTrait) -> TokenStream {
    let mut extern_methods = quote!{};
    let mut struct_members = quote!{};
    let mut initializers   = quote!{};
    let trait_ident  = &trait_.ident;

    for item in &trait_.items {
        match item {
            TraitItem::Method(method) => {
                let ident = &method.sig.ident;
                let ret = &method.sig.decl.output;
                let (args, method, keep_first) =
                    convert_self(&method.sig.decl.inputs);
                let call_args = arg_idents(&args, keep_first);

                extern_methods = extern_methods.append (quote! {
                    pub extern fn #ident <T: #trait_ident> (#args) # ret {
                        unsafe { #method #ident (#call_args) }
                    }
                });
                struct_members = struct_members.append (quote!{
                    #ident : extern fn (#args) #ret,
                });
                initializers = initializers.append (quote!{
                    #ident : #ident::<T>,
                });
            }
            _ => ()
        }
    }
    let mod_= trait_ident.clone().prepend("ability_");
    let vtable = trait_ident.clone().append("VTable");
    let expanded = quote! {
    #trait_

    #[allow(non_snake_case)]
    #[allow(dead_code)]
    pub mod #mod_ {
        use super::#trait_ident;

        #extern_methods

        #[repr(C)]
        pub struct #vtable {
            #struct_members
        }

        impl #vtable {
            pub fn new <T: #trait_ident>() -> Self {
                Self { #initializers }
            }
        }
    }
    };
    println!("{}", expanded);

    TokenStream::from (expanded)
}

// maps &self/&mut self args to *const c_void/*mut c_void. The additional token stream result is
// used to wrap the method calls.
fn convert_self (args : &Punctuated<FnArg, Comma>) -> (Punctuated<FnArg, Comma>, TStream, bool) {
    let mut lookup = quote!(T::);
    let mut keep_first = true;
    let it = args
        .iter()
        .map(|arg|{
            match arg {
                FnArg::SelfRef(self_ref) => {
                    keep_first = false;
                    let sig =
                        if self_ref.mutability.is_some() {
                            lookup = quote!( (* (self_ as *mut T) ) . );
                            quote!(*mut std::os::raw::c_void)
                        }
                        else {
                            lookup = quote!( (* (self_ as *const T)) . );
                            quote!(*const std::os::raw::c_void)
                        };

                    FnArg::Captured(syn::ArgCaptured {
                        pat         : syn::parse(quote!(self_).into()).unwrap(),
                        colon_token : syn::parse(quote!(:).into()).unwrap(),
                        ty          : syn::parse(sig.into()).unwrap()
                    })
                },
                _ => arg.clone()
            }
        });
    (Punctuated::from_iter(it), lookup, keep_first)
}

fn arg_idents (args : &Punctuated<FnArg, Comma>, keep_first : bool) -> Punctuated <Pat, Comma> {
    let it = args
        .iter()
        .skip(if keep_first {0} else {1})
        .map(|arg|
            match arg {
                FnArg::Captured(captured) => captured.pat.clone(),
                _ => panic!("no inferred type, &self or Self arguments supported")
            })
        .into_iter();

    Punctuated::from_iter(it)
}

fn struct_interface(_ : &TokenStream, _ : &ItemStruct) -> TokenStream {
    unimplemented!()
}

trait Append <T> {
    fn append(self, suffix : T) -> Self;
    fn prepend(self, prefix : T) -> Self;
}

impl<'a> Append<&'a str> for syn::Ident {
    fn append (self, suffix : &str) -> Self {
        let s = format!("{}{}", self, suffix);
        syn::Ident::new(&s, self.span())
    }

    fn prepend (self, prefix : &str) -> Self {
        let s = format!("{}{}", prefix, self);
        syn::Ident::new(&s, self.span())
    }
}

impl Append<TStream> for TStream {
    fn append(mut self, suffix : TStream) -> Self {
        self.extend(suffix);
        self
    }

    fn prepend(self, mut prefix : TStream) -> Self {
        prefix.extend (self);
        prefix
    }
}