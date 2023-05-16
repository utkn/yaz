use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::ItemFn;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn tx_generator(_args: TokenStream, tagged_fn: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tagged_fn as ItemFn);
    let fn_name = input.sig.ident.clone();
    let const_name = Ident::new(&fn_name.to_string().to_uppercase(), Span::call_site());
    let expanded = quote! {
        pub const #const_name: crate::editor::TransactionGenerator
            = crate::editor::TransactionGenerator(std::stringify!(#fn_name), #fn_name);
        #input
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn action_generator(_args: TokenStream, tagged_fn: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tagged_fn as ItemFn);
    let fn_name = input.sig.ident.clone();
    let const_name = Ident::new(&fn_name.to_string().to_uppercase(), Span::call_site());
    let expanded = quote! {
        pub const #const_name: crate::editor::ActionGenerator
            = crate::editor::ActionGenerator(std::stringify!(#fn_name), #fn_name);
        #input
    };
    TokenStream::from(expanded)
}

#[proc_macro_derive(BasicEditorMode, attributes(handler))]
pub fn create_basic_editor_mode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident.clone();
    let mode_id = struct_name.to_string().to_lowercase().replace("mode", "");
    let mode_id = Ident::new(&mode_id, Span::call_site());
    let expanded = quote! {
        impl #struct_name {
            pub fn id() -> &'static str {
                std::stringify!(#mode_id)
            }
        }

        impl crate::editor::editor_mode::EditorMode for #struct_name {
            fn id(&self) -> &'static str {
                Self::id()
            }

            fn handle_combo(&mut self, kc: &crate::events::KeyCombo, _state: &crate::editor::EditorStateSummary)
                -> crate::editor::EditorAction {
                self.trigger_handler.handle(kc).unwrap_or_default()
            }

            fn get_display(&self, _: &crate::editor::EditorStateSummary) -> crate::editor::EditorDisplay {
                Default::default()
            }
        }
    };
    TokenStream::from(expanded)
}
