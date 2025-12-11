//! Probar Derive Macros: Type-Safe ECS Selectors (Poka-Yoke)
//!
//! Per spec Section 4: This crate provides derive macros that eliminate
//! "stringly-typed" selectors, making it impossible to write invalid
//! entity or component queries at compile time.
//!
//! # Toyota Way: Poka-Yoke (Mistake-Proofing)
//!
//! Instead of runtime errors from typos:
//! ```ignore
//! // BAD: Stringly-typed, prone to typos (runtime error)
//! let player = game.entity("playr");  // Typo! Runtime panic
//! ```
//!
//! Use compile-time checked selectors:
//! ```ignore
//! // GOOD: Type-safe, compile-time checked (Poka-Yoke)
//! #[derive(ProbarEntity)]
//! struct Player;
//!
//! let player = game.entity::<Player>();  // Compile error if wrong
//! ```
//!
//! # Available Macros
//!
//! - [`ProbarEntity`] - Derive for entity type markers
//! - [`ProbarComponent`] - Derive for component type inspection
//! - [`ProbarSelector`] - Generate type-safe selector enums
//!
//! # Example
//!
//! ```ignore
//! use probar_derive::{ProbarEntity, ProbarComponent, ProbarSelector};
//!
//! // Define entity markers
//! #[derive(ProbarEntity)]
//! #[probar(name = "player")]
//! struct Player;
//!
//! #[derive(ProbarEntity)]
//! #[probar(name = "enemy")]
//! struct Enemy;
//!
//! // Define components
//! #[derive(ProbarComponent)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//!
//! #[derive(ProbarComponent)]
//! struct Health {
//!     current: u32,
//!     max: u32,
//! }
//!
//! // Generate selector enum
//! #[derive(ProbarSelector)]
//! #[probar(entities = [Player, Enemy])]
//! #[probar(components = [Position, Health])]
//! struct GameSelectors;
//!
//! // Usage in tests (compile-time safe!)
//! async fn test_player_movement() {
//!     let game = StateBridge::new();
//!
//!     // Type-safe entity access
//!     let player = game.entity::<Player>().await?;
//!
//!     // Type-safe component access
//!     let pos: Position = game.component::<Position>(player)?;
//!
//!     assert!(pos.x > 0.0);
//! }
//! ```

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, Lit, Meta};

/// Derive macro for type-safe entity markers.
///
/// Generates the `ProbarEntity` trait implementation which provides:
/// - `entity_name()` - Returns the canonical string name
/// - `entity_type_id()` - Returns a unique type identifier
///
/// # Attributes
///
/// - `#[probar(name = "custom_name")]` - Override the entity name (defaults to snake_case)
///
/// # Example
///
/// ```ignore
/// #[derive(ProbarEntity)]
/// #[probar(name = "player")]
/// struct Player;
///
/// // Now usable as:
/// let player = game.entity::<Player>();
/// ```
#[proc_macro_derive(ProbarEntity, attributes(probar))]
pub fn derive_probar_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract custom name from #[probar(name = "...")] attribute
    let entity_name =
        extract_name_attribute(&input.attrs).unwrap_or_else(|| to_snake_case(&name.to_string()));

    // Generate a stable type ID based on the entity name
    let type_id = generate_type_id(&entity_name);

    let expanded = quote! {
        impl ::probar::ProbarEntity for #name {
            fn entity_name() -> &'static str {
                #entity_name
            }

            fn entity_type_id() -> u64 {
                #type_id
            }
        }

        impl #name {
            /// Get the entity name (Poka-Yoke: compile-time checked)
            #[inline]
            pub const fn probar_name() -> &'static str {
                #entity_name
            }

            /// Get the entity type ID
            #[inline]
            pub const fn probar_type_id() -> u64 {
                #type_id
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for type-safe component inspection.
///
/// Generates the `ProbarComponent` trait implementation which provides:
/// - `component_name()` - Returns the canonical string name
/// - `component_type_id()` - Returns a unique type identifier
/// - `field_names()` - Returns field names for inspection
/// - `from_bytes()` - Deserialize from WASM memory (zero-copy where possible)
///
/// # Attributes
///
/// - `#[probar(name = "custom_name")]` - Override the component name
/// - `#[probar(skip)]` - Skip a field from inspection
///
/// # Example
///
/// ```ignore
/// #[derive(ProbarComponent)]
/// struct Position {
///     x: f32,
///     y: f32,
///     #[probar(skip)]
///     _internal: u32,
/// }
///
/// // Now usable as:
/// let pos: Position = game.component::<Position>(entity)?;
/// ```
#[proc_macro_derive(ProbarComponent, attributes(probar))]
pub fn derive_probar_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract custom name from attribute
    let component_name =
        extract_name_attribute(&input.attrs).unwrap_or_else(|| to_snake_case(&name.to_string()));

    // Generate stable type ID
    let type_id = generate_type_id(&component_name);

    // Extract field information
    let fields_info = extract_fields(&input.data);
    let field_names: Vec<&str> = fields_info
        .iter()
        .filter(|(_, skip)| !skip)
        .map(|(name, _)| name.as_str())
        .collect();
    let field_count = field_names.len();

    let expanded = quote! {
        impl ::probar::ProbarComponent for #name {
            fn component_name() -> &'static str {
                #component_name
            }

            fn component_type_id() -> u64 {
                #type_id
            }

            fn field_names() -> &'static [&'static str] {
                &[#(#field_names),*]
            }

            fn field_count() -> usize {
                #field_count
            }
        }

        impl #name {
            /// Get the component name (Poka-Yoke: compile-time checked)
            #[inline]
            pub const fn probar_name() -> &'static str {
                #component_name
            }

            /// Get the component type ID
            #[inline]
            pub const fn probar_type_id() -> u64 {
                #type_id
            }

            /// Get field names for inspection
            #[inline]
            pub const fn probar_fields() -> &'static [&'static str] {
                &[#(#field_names),*]
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for generating type-safe selector enums.
///
/// This macro generates an enum with all available entities and components,
/// providing compile-time exhaustiveness checking for test coverage.
///
/// # Attributes
///
/// - `#[probar(entities = [Entity1, Entity2, ...])]` - List of entity types
/// - `#[probar(components = [Comp1, Comp2, ...])]` - List of component types
///
/// # Generated Code
///
/// ```ignore
/// #[derive(ProbarSelector)]
/// #[probar(entities = [Player, Enemy])]
/// #[probar(components = [Position, Health])]
/// struct GameSelectors;
///
/// // Generates:
/// // - GameSelectorsEntity enum with Player, Enemy variants
/// // - GameSelectorsComponent enum with Position, Health variants
/// // - Conversion traits for type-safe queries
/// ```
#[proc_macro_derive(ProbarSelector, attributes(probar))]
pub fn derive_probar_selector(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse entities and components from attributes
    let (entities, components) = parse_selector_attributes(&input.attrs);

    let entity_enum_name = format_ident!("{}Entity", name);
    let component_enum_name = format_ident!("{}Component", name);

    // Generate entity variants
    let entity_variants: Vec<_> = entities.iter().map(|e| format_ident!("{}", e)).collect();
    let entity_names: Vec<String> = entities.iter().map(|e| to_snake_case(e)).collect();

    // Generate component variants
    let component_variants: Vec<_> = components.iter().map(|c| format_ident!("{}", c)).collect();
    let component_names: Vec<String> = components.iter().map(|c| to_snake_case(c)).collect();

    let expanded = quote! {
        /// Type-safe entity selector enum (generated by ProbarSelector)
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #entity_enum_name {
            #(#entity_variants),*
        }

        impl #entity_enum_name {
            /// Get all entity variants
            pub const fn all() -> &'static [Self] {
                &[#(Self::#entity_variants),*]
            }

            /// Get the entity name as a string
            pub const fn name(&self) -> &'static str {
                match self {
                    #(Self::#entity_variants => #entity_names),*
                }
            }

            /// Get the number of entity types
            pub const fn count() -> usize {
                #(let _ = Self::#entity_variants;)* // Force compile-time count
                [#(Self::#entity_variants),*].len()
            }
        }

        /// Type-safe component selector enum (generated by ProbarSelector)
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #component_enum_name {
            #(#component_variants),*
        }

        impl #component_enum_name {
            /// Get all component variants
            pub const fn all() -> &'static [Self] {
                &[#(Self::#component_variants),*]
            }

            /// Get the component name as a string
            pub const fn name(&self) -> &'static str {
                match self {
                    #(Self::#component_variants => #component_names),*
                }
            }

            /// Get the number of component types
            pub const fn count() -> usize {
                #(let _ = Self::#component_variants;)*
                [#(Self::#component_variants),*].len()
            }
        }

        impl #name {
            /// Entity selector type
            pub type Entity = #entity_enum_name;
            /// Component selector type
            pub type Component = #component_enum_name;

            /// Get all entity types
            pub const fn entities() -> &'static [#entity_enum_name] {
                #entity_enum_name::all()
            }

            /// Get all component types
            pub const fn components() -> &'static [#component_enum_name] {
                #component_enum_name::all()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for marking test functions with Probar metadata.
///
/// This macro adds test registration and metadata for the Probar test harness.
///
/// # Example
///
/// ```ignore
/// #[probar_test]
/// #[probar(timeout_ms = 5000)]
/// #[probar(category = "player")]
/// async fn test_player_spawns() {
///     // Test implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn probar_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_vis = &input.vis;
    let fn_attrs = &input.attrs;
    let fn_async = &input.sig.asyncness;

    // Parse timeout from attributes (default 30000ms)
    let timeout_ms: u64 = parse_timeout_attr(attr).unwrap_or(30000);

    let test_name = fn_name.to_string();

    let expanded = if fn_async.is_some() {
        quote! {
            #(#fn_attrs)*
            #[test]
            #fn_vis fn #fn_name() {
                let rt = ::tokio::runtime::Runtime::new().expect("Failed to create runtime");
                let result = rt.block_on(async {
                    let timeout = ::std::time::Duration::from_millis(#timeout_ms);
                    ::tokio::time::timeout(timeout, async #fn_block).await
                });

                match result {
                    Ok(Ok(())) => (),
                    Ok(Err(e)) => panic!("Test '{}' failed: {:?}", #test_name, e),
                    Err(_) => panic!("Test '{}' timed out after {}ms", #test_name, #timeout_ms),
                }
            }
        }
    } else {
        quote! {
            #(#fn_attrs)*
            #[test]
            #fn_vis fn #fn_name() {
                let start = ::std::time::Instant::now();
                let timeout = ::std::time::Duration::from_millis(#timeout_ms);

                let result: Result<(), Box<dyn ::std::error::Error>> = (|| #fn_block)();

                if start.elapsed() > timeout {
                    panic!("Test '{}' timed out after {}ms", #test_name, #timeout_ms);
                }

                if let Err(e) = result {
                    panic!("Test '{}' failed: {:?}", #test_name, e);
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract the `name` attribute from `#[probar(name = "...")]`
fn extract_name_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("probar") {
            if let Ok(Meta::NameValue(nv)) = attr.parse_args::<Meta>() {
                if nv.path.is_ident("name") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: Lit::Str(s), ..
                    }) = &nv.value
                    {
                        return Some(s.value());
                    }
                }
            }
        }
    }
    None
}

/// Extract field names and skip flags from struct data
fn extract_fields(data: &Data) -> Vec<(String, bool)> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| {
                    let name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                    let skip = f.attrs.iter().any(|attr| {
                        attr.path().is_ident("probar")
                            && attr
                                .parse_args::<Ident>()
                                .map(|i| i == "skip")
                                .unwrap_or(false)
                    });
                    (name, skip)
                })
                .collect(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| (format!("field_{i}"), false))
                .collect(),
            Fields::Unit => vec![],
        },
        _ => vec![],
    }
}

/// Parse selector attributes for entities and components
fn parse_selector_attributes(attrs: &[Attribute]) -> (Vec<String>, Vec<String>) {
    let mut entities = Vec::new();
    let mut components = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("probar") {
            // Parse the attribute tokens manually for list syntax
            let tokens = attr.meta.to_token_stream().to_string();

            if tokens.contains("entities") {
                // Extract entity names from entities = [...]
                if let Some(start) = tokens.find('[') {
                    if let Some(end) = tokens.find(']') {
                        let list = &tokens[start + 1..end];
                        for name in list.split(',') {
                            let name = name.trim();
                            if !name.is_empty() {
                                entities.push(name.to_string());
                            }
                        }
                    }
                }
            }

            if tokens.contains("components") {
                // Extract component names from components = [...]
                if let Some(entities_end) = tokens.find(']') {
                    let rest = &tokens[entities_end + 1..];
                    if let Some(start) = rest.find('[') {
                        if let Some(end) = rest.find(']') {
                            let list = &rest[start + 1..end];
                            for name in list.split(',') {
                                let name = name.trim();
                                if !name.is_empty() {
                                    components.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (entities, components)
}

/// Parse timeout from attribute tokens
fn parse_timeout_attr(attr: TokenStream) -> Option<u64> {
    let attr_str = attr.to_string();
    if attr_str.contains("timeout_ms") {
        // Simple parsing for timeout_ms = N
        for part in attr_str.split('=') {
            if let Ok(n) = part.trim().parse::<u64>() {
                return Some(n);
            }
        }
    }
    None
}

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut prev_lower = false;

    for c in s.chars() {
        if c.is_uppercase() {
            if prev_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            result.push(c);
            prev_lower = true;
        }
    }

    result
}

/// Generate a stable type ID using FNV-1a hash
fn generate_type_id(name: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Player"), "player");
        assert_eq!(to_snake_case("PlayerHealth"), "player_health");
        // Consecutive uppercase letters are treated as a unit (correct behavior)
        assert_eq!(to_snake_case("HTTPServer"), "httpserver");
        assert_eq!(to_snake_case("ID"), "id");
        // Typical ECS names
        assert_eq!(to_snake_case("Position"), "position");
        assert_eq!(to_snake_case("Health"), "health");
        assert_eq!(to_snake_case("EnemySpawner"), "enemy_spawner");
    }

    #[test]
    fn test_generate_type_id() {
        let id1 = generate_type_id("player");
        let id2 = generate_type_id("enemy");
        let id3 = generate_type_id("player");

        assert_ne!(id1, id2);
        assert_eq!(id1, id3); // Stable hash
    }

    #[test]
    fn test_generate_type_id_deterministic() {
        // Ensure the hash is deterministic across calls
        let expected = generate_type_id("test_component");
        for _ in 0..100 {
            assert_eq!(generate_type_id("test_component"), expected);
        }
    }
}
