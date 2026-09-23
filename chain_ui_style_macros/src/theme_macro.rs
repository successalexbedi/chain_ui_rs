use crate::cursor::Cursor;
use proc_macro2::{Delimiter, TokenStream};
use quote::{format_ident, quote};

fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);
    let name_lit = cur.expect_literal();
    let theme_name = name_lit.to_string().trim_matches('"').to_string();

    let body_group = cur.expect_group(Delimiter::Brace);
    let mut body = Cursor::new(body_group.stream());

    let mut markers = Vec::new();
    let mut keyframe_fns = Vec::new();
    let mut include_globals = false;

    while !body.eof() {
        let item_name = body.expect_ident();

        if item_name == "keyframes" {
            body.expect_punct(':');
            loop {
                let kf_name = body.expect_ident();
                keyframe_fns.push(format_ident!("{}_keyframes", kf_name));
                if body.peek_is_punct(',') { body.bump(); continue; }
                break;
            }
            body.expect_punct(';');
            continue;
        }

        body.expect_punct(';');
        if item_name == "global" {
            include_globals = true;
        } else {
            markers.push(format_ident!("{}", snake_to_pascal(&item_name.to_string())));
        }
    }

    let fn_name = format_ident!("{}_theme", theme_name);
    let css_fn_name = format_ident!("{}_css", theme_name);
    let build_fn_name = format_ident!("__build_{}_css", theme_name);

    let global_extend = if include_globals {
        quote! { styles.extend(__global_styles()); }
    } else {
        quote! {}
    };

    quote! {
        #[cold]
        fn #build_fn_name() -> String {
            let mut styles: Vec<chain_ui_style::ast::Style> = Vec::new();
            #global_extend
            #( styles.push(<#markers as chain_ui_style::registry::StyleDef>::build()); )*

            let mut css = chain_ui_style::render::render_css(styles);
            #( css.push_str(&chain_ui_style::render::render_keyframes(&#keyframe_fns())); )*

            // dev builds stay readable for debugging via /__css;
            // release builds ship minified automatically, no
            // separate feature flag or config needed
            if cfg!(debug_assertions) {
                css
            } else {
                chain_ui_style::render::minify(&css)
            }
        }

        #[inline]
        pub fn #css_fn_name() -> &'static str {
            static CSS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            CSS.get_or_init(#build_fn_name)
        }

        #[inline]
        
pub fn #fn_name() -> chain_ui_core::Element {
    chain_ui_core::tag::style().child(chain_ui_core::raw_html(#css_fn_name()))
}
    }
}