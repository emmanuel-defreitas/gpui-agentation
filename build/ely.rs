//! Build-only namespace adaptation of the immutable upstream Ely snapshot.
use std::{env, fs, path::Path};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Adapt paths as tokens, preserving comments, literals, formatting and line numbers.
/// Upstream tests remain disabled, as they were when Ely was an external dependency.
pub fn adapt(source: &str, assets: bool) -> String {
    let source = if assets {
        assert_eq!(source.matches("#[folder = \"assets\"]").count(), 1);
        source.replace(
            "#[folder = \"assets\"]",
            "#[folder = \"vendor/ely/assets\"]",
        )
    } else {
        source.to_owned()
    };
    let tokens: TokenStream = source.parse().expect("pinned Ely source must tokenize");
    let mut edits = Vec::new();
    collect(tokens, false, &mut edits);
    edits.sort_unstable_by_key(|(start, _, _)| *start);
    let mut result = source;
    for (start, end, replacement) in edits.into_iter().rev() {
        result.replace_range(start..end, replacement);
    }
    result
}

fn ident(token: Option<&TokenTree>, name: &str) -> bool {
    matches!(token, Some(TokenTree::Ident(value)) if value == name)
}

fn punct(token: Option<&TokenTree>, character: char) -> bool {
    matches!(token, Some(TokenTree::Punct(value)) if value.as_char() == character)
}

fn collect(tokens: TokenStream, cfg: bool, edits: &mut Vec<(usize, usize, &'static str)>) {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(value)
                if value == "crate"
                    && punct(tokens.get(index + 1), ':')
                    && punct(tokens.get(index + 2), ':') =>
            {
                let end = value.span().byte_range().end;
                edits.push((end, end, "::ely"));
            }
            TokenTree::Ident(value) if cfg && value == "test" => {
                let range = value.span().byte_range();
                edits.push((range.start, range.end, "any()"));
            }
            TokenTree::Group(group) => {
                let previous = index.checked_sub(1).and_then(|i| tokens.get(i));
                let cfg_group = group.delimiter() == Delimiter::Parenthesis
                    && (ident(previous, "cfg")
                        || (punct(previous, '!')
                            && ident(index.checked_sub(2).and_then(|i| tokens.get(i)), "cfg")));
                collect(group.stream(), cfg || cfg_group, edits);
            }
            _ => {}
        }
    }
}

pub fn generate() {
    println!("cargo:rerun-if-changed=vendor/ely/src");
    println!("cargo:rerun-if-changed=vendor/ely/assets");
    let output = Path::new(&env::var_os("OUT_DIR").expect("Cargo OUT_DIR")).join("ely");
    copy_sources(Path::new("vendor/ely/src"), &output);
}

fn copy_sources(source: &Path, output: &Path) {
    fs::create_dir_all(output).expect("create generated Ely directory");
    let mut entries: Vec<_> = fs::read_dir(source)
        .expect("read pinned Ely directory")
        .map(|entry| entry.expect("read pinned Ely file").path())
        .collect();
    entries.sort();
    for path in entries {
        let target = output.join(path.file_name().expect("Ely file name"));
        if path.is_dir() {
            copy_sources(&path, &target);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path).expect("read pinned Ely source");
            let text = adapt(&text, path == Path::new("vendor/ely/src/assets.rs"));
            fs::write(target, text).expect("write adapted Ely source");
        } else {
            fs::copy(&path, target).expect("copy pinned Ely resource");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::adapt;

    #[test]
    fn namespaces_paths_in_imports_and_macros_without_touching_text() {
        let source = "// crate::unchanged 🟠\nuse crate::{theme, forms};\ncall!(crate::theme::Color);\nconst TEXT: &str = r#\"crate::theme\"#;\n";
        let expected = "// crate::unchanged 🟠\nuse crate::ely::{theme, forms};\ncall!(crate::ely::theme::Color);\nconst TEXT: &str = r#\"crate::theme\"#;\n";
        assert_eq!(adapt(source, false), expected);
    }

    #[test]
    fn disables_dependency_tests_but_preserves_test_support_and_literals() {
        let source = "#[cfg(any(test, feature = \"test-support\"))]\npub fn setup() {}\n#[cfg(test)] mod tests {}\nconst TEST: bool = cfg!(test);\n#[cfg(feature = \"test\")] fn other() {}\n";
        let expected = "#[cfg(any(any(), feature = \"test-support\"))]\npub fn setup() {}\n#[cfg(any())] mod tests {}\nconst TEST: bool = cfg!(any());\n#[cfg(feature = \"test\")] fn other() {}\n";
        assert_eq!(adapt(source, false), expected);
    }

    #[test]
    fn assets_resolve_from_the_published_package_root() {
        assert_eq!(
            adapt("#[folder = \"assets\"]\npub struct Assets;\n", true),
            "#[folder = \"vendor/ely/assets\"]\npub struct Assets;\n"
        );
    }
}
