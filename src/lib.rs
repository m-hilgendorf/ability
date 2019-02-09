#![allow(dead_code)]

extern crate proc_macro;
use proc_macro::TokenStream;
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
        Item::Trait(trait_) => trait_interface(trait_),
        Item::Struct(struct_) => struct_interface(attr, struct_),
        _ => cloned
    }
  //  cloned
}

fn trait_interface (trait_ : ItemTrait) -> TokenStream {
    let mut extern_methods = quote!{};
    let mut struct_members = quote!{};
    let mut initializers   = quote!{};
    let trait_ident  = trait_.ident.clone();
    let cloned = trait_.clone();

    for item in &cloned.items {
        match item {
            TraitItem::Method(method) => {
                let sig = &method.sig;
                let ident = &sig.ident;
                let decl = &sig.decl;
                let (args, lookup) = convert_self(&decl.inputs);
                let call_args = arg_idents(&args, lookup == MethodLookup::Static);
                let ret = &decl.output;

                let method_lookup = match lookup {
                    MethodLookup::Static => quote! {
                        T::#ident ( #call_args )
                    },
                    MethodLookup::ConstSelf => quote! {
                        (* (self_ as *const T)).#ident ( #call_args )
                    },
                    MethodLookup::MutSelf => quote! {
                        (* (self_ as *mut T)).#ident ( #call_args )
                    },
                };

                extern_methods = quote! {
                    #extern_methods
                    pub unsafe extern fn #ident <T: #trait_ident> (#args) #ret
                    {    #method_lookup   }
                };
                struct_members = quote! {
                    #struct_members
                    #ident : unsafe extern fn ( #args ) #ret,
                };
                initializers = quote! {
                    #initializers
                    #ident: #ident::<T>,
                };
            }
            _ => ()
        }
    }

    let mod_ident    = format!("ability_{}", trait_ident);
    let mod_ident = syn::Ident::new(&mod_ident, trait_ident.span());
    let vtable_ident = format!("{}VTable", trait_ident);
    let vtable_ident = syn::Ident::new(&vtable_ident, trait_ident.span());
    let expanded = quote! {
    #trait_

    #[allow(non_snake_case)]
    #[allow(dead_code)]
    pub mod #mod_ident {
        use super::#trait_ident;

        #extern_methods

        #[repr(C)]
        pub struct #vtable_ident {
            #struct_members
        }

        impl #vtable_ident {
            pub fn new <T: #trait_ident>() -> Self {
                Self { #initializers }
            }
        }
    }
    };
    TokenStream::from (expanded)
}
#[derive(Eq,PartialEq)]
enum MethodLookup { ConstSelf, MutSelf, Static }

//todo: converting arguments could be done cleaner
fn convert_self (args : &Punctuated<FnArg, Comma>) -> (Punctuated<FnArg, Comma>, MethodLookup) {
    let mut new_args = Punctuated::new();
    let mut lookup = MethodLookup::Static;

    for arg in args.iter() {
        match arg {
            FnArg::SelfRef(self_ref) => {
                let sig =
                    if self_ref.mutability.is_some() { quote!(*mut std::os::raw::c_void)}
                    else {quote!(*const std::os::raw::c_void)};

                let new_arg = syn::ArgCaptured {
                    pat         : syn::parse (quote!(self_).into()).unwrap(),
                    colon_token : syn::parse (quote!(:).into()).unwrap(),
                    ty          : syn::parse(sig.into()).unwrap()
                };

                //let new_arg = syn::parse (tokens).expect("failed to parse tokens");
                new_args.push (FnArg::Captured(new_arg));
                lookup = if self_ref.mutability.is_some() { MethodLookup::MutSelf}
                         else { MethodLookup::ConstSelf };
            },
            _ => new_args.push(arg.clone())
        }
    }
    (new_args, lookup)
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

fn struct_interface(_ : TokenStream, _ : ItemStruct) -> TokenStream {
    unimplemented!()
}
