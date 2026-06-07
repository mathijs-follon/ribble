use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, GenericParam, Generics, Meta};

fn add_send_sync_static_bounds(mut generics: Generics) -> Generics {
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(syn::parse_quote!(::core::marker::Send));
            type_param
                .bounds
                .push(syn::parse_quote!(::core::marker::Sync));
            type_param
                .bounds
                .push(syn::parse_quote!('static));
        }
    }
    generics
}

fn has_list_flag(attrs: &[Attribute], attr_name: &str, flag: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident(attr_name) {
            return false;
        }
        let Meta::List(list) = &attr.meta else {
            return false;
        };
        list.parse_args_with(
            syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
        )
        .is_ok_and(|items| {
            items
                .iter()
                .any(|meta| matches!(meta, Meta::Path(path) if path.is_ident(flag)))
        })
    })
}

/// Derive macro for the [`Component`](https://docs.rs/ribble_ecs/latest/ribble_ecs/trait.Component.html) trait.
///
/// Also implements [`RegisterComponent`](https://docs.rs/ribble_ecs/latest/ribble_ecs/trait.RegisterComponent.html)
/// unless `#[component(no_register)]` is set.
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let no_register = has_list_flag(&input.attrs, "component", "no_register");
    let generics = add_send_sync_static_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let register_impl = if no_register {
        quote! {}
    } else {
        quote! {
            impl #impl_generics ::ribble_ecs::RegisterComponent for #name #ty_generics #where_clause {
                fn register(world: &mut ::ribble_ecs::World) {
                    world.register_component::<Self>();
                }
            }
        }
    };

    quote! {
        impl #impl_generics ::ribble_ecs::Component for #name #ty_generics #where_clause {}

        #register_impl
    }
    .into()
}

/// Derive macro for the [`Event`](https://docs.rs/ribble_ecs/latest/ribble_ecs/trait.Event.html) trait.
///
/// Also implements [`RegisterEvent`](https://docs.rs/ribble_ecs/latest/ribble_ecs/trait.RegisterEvent.html)
/// unless `#[event(no_register)]` is set.
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let no_register = has_list_flag(&input.attrs, "event", "no_register");
    let generics = add_send_sync_static_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let register_impl = if no_register {
        quote! {}
    } else {
        quote! {
            impl #impl_generics ::ribble_ecs::RegisterEvent for #name #ty_generics #where_clause {
                fn register(world: &mut ::ribble_ecs::World) {
                    world.add_event::<Self>();
                }
            }
        }
    };

    quote! {
        impl #impl_generics ::ribble_ecs::Event for #name #ty_generics #where_clause {}

        #register_impl
    }
    .into()
}

/// Derive macro for the [`Resource`](https://docs.rs/ribble_ecs/latest/ribble_ecs/trait.Resource.html) trait.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = add_send_sync_static_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ::ribble_ecs::Resource for #name #ty_generics #where_clause {}
    }
    .into()
}
