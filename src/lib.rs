/*!
    ability (**ABI** compati**bility**) is a crate for ABI compatibility via traits. The intended
    use is to allow application developers to specify plugin/extension APIs using traits.

    ## Usage ##

    Add the `#[interface]` attribute to a trait you wwant to
    ```rust
    use ability::interface;

    #[interface]
    trait Trait {
        fn foo(&self);
        fn bar(&mut self);
        fn baz();
    }
    ```

    ## How it works ##

    The above code generates a [virtual method table]() (vtable), which looks like this:

    ```rust
    #[allow(non_snake_case)]
    #[allow(dead_code )]
    pub mod ability_Trait {
        use super::Trait;

        pub extern fn foo <T : Trait> (self_ : * const std::os::raw::c_void) {
            unsafe { (*(self_ as *const T)).foo() }
        }
        pub extern fn bar <T : Trait> (self_ : * mut std::os::raw::c_void) {
            unsafe { (*(self_ as *mut T)).bar() }
        }
        pub extern fn baz <T : Trait>() {
            unsafe { T :: baz() }
         }

        #[repr(C)]
        pub struct
        TraitVTable {
            foo : extern fn (self_ : *const std::os::raw::c_void),
            bar : extern fn (self_ : *mut std::os::raw::c_void),
            baz : extern fn () ,
        }

        impl TraitVTable {
            pub fn new <T : Trait> () -> Self {
                Self {
                    foo : foo::<T>,
                    bar : bar::<T>,
                    baz : baz::<T>,
                }
            }
        }
    }
    ```

    Generated code is placed in a new module which imports the wrapped trait. FFI methods (`extern`)
    methods are declared that are generic over types that implement the wrapped trait.

    The vtable is generic over types that implement the wrapped trait, and a constructor method is
    provided to generate the vtable at runtime.

    This is similar to how dynamic dispatch works "under the hood." This does not seek to replace
    dynamic dispatch, just provide functionality across library boundaries that are compiled with
    different compiler versions (e.g., plugins and application extensions distributed as shared
    libraries).

    ## On going work ##

    - Limiting types that can pass across interface boundaries, to prevent subtle bugs from attempting
    to use incompatible types.

    - A sensible API for wrapping structs that implement the traits

    - Generator/factory pattern to create a single entry point into a shared library
*/
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