// Helper macro to distinguish between patterns and expressions
#[macro_export]
macro_rules! assert_case {
    // Guard patterns - patterns with if conditions
    ($result:expr, { $pattern:pat if $guard:expr }) => {
        match $result {
            $pattern if $guard => {},
            other => panic!("Expected {} if {} but got {:?}", stringify!($pattern), stringify!($guard), other),
        }
    };

    // Pattern matching cases - detect common pattern forms
    ($result:expr, { Ok($($pattern:tt)*) }) => {
        match $result {
            Ok($($pattern)*) => {},
            other => panic!("Expected Ok({}) but got {:#?}", stringify!($($pattern)*), other),
        }
    };

    ($result:expr, { Err($($pattern:tt)*) }) => {
        match $result {
            Err($($pattern)*) => {},
            other => panic!("Expected Err({}) but got {:#?}", stringify!($($pattern)*), other),
        }
    };

    ($result:expr, { Some($($pattern:tt)*) }) => {
        match $result {
            Some($($pattern)*) => {},
            other => panic!("Expected Some({}) but got {:#?}", stringify!($($pattern)*), other),
        }
    };

    ($result:expr, { None }) => {
        match $result {
            None => {},
            other => panic!("Expected None but got {:#?}", other),
        }
    };

    // Wildcard pattern
    ($result:expr, { _ }) => {
        // Always passes - just to ensure the expression compiles
        let _ = $result;
    };

    // Default case - treat as expression for equality comparison
    ($result:expr, { $expected:expr }) => {
        match $result {
            Ok(actual) => {
                pretty_assertions::assert_eq!($expected, actual, "Expected {:#?} but got {:#?}\n\n< expected / got >", $expected, actual);
            },
            other => panic!("Expected Ok(...) but got {:?}", other),
        }
    };
}

// Helper macro to generate test functions based on field names
#[macro_export]
macro_rules! handle_case {
    // With attributes (including empty attribute list)
    ([$($attrs:meta)*] ast, $expected:tt) => {
        $(#[$attrs])*
        #[test]
        fn validate_ast() {
            let arena = bumpalo::Bump::new();
            let result = melbi_core::parser::parse(&arena, input()).map(|p| p.expr);
            assert_case!(result, $expected);
        }
    };

    ([$($attrs:meta)*] formatted, $expected:tt) => {
        $(#[$attrs])*
        #[test]
        fn validate_formatted() {
            let formatted = melbi_fmt::format(input(), false, false);
            let result = formatted
                .as_ref()
                .map(|s| s.as_str())
                .map_err(|e| e.downcast_ref::<melbi_fmt::FormatError>().unwrap());

            if let Err(melbi_fmt::FormatError::Idempotency) = &result {
                eprintln!("{}", result.unwrap_err());
                if let Ok(first) = melbi_fmt::format(input(), true, false) {
                    println!("FIRST:\n{}", first);
                    if let Ok(second) = melbi_fmt::format(&first, true, false) {
                        println!("SECOND:\n{}", second);
                    }
                }
            }
            assert_case!(result, $expected);
        }
    };

    ([$($attrs:meta)*] error, $expected:tt) => {
        $(#[$attrs])*
        #[test]
        fn validate_error() {
            // Normalize by stripping trailing whitespace from each line
            fn normalize(s: &str) -> String {
                s.lines()
                    .map(|line| line.trim_end())
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            }

            let arena = bumpalo::Bump::new();
            let engine = melbi::Engine::new(melbi::EngineOptions::default(), &arena, |_, _, env| env);
            let result = engine.compile(Default::default(), input(), &[]);

            let err = match result {
                Err(e) => e,
                Ok(_) => panic!("Expected compilation error, but compilation succeeded"),
            };
            let mut buf = Vec::new();
            let config = melbi::RenderConfig { color: false, ..Default::default() };
            melbi::render_error_to(&err, &mut buf, &config).unwrap();
            let err_string = String::from_utf8_lossy(&buf).into_owned();
            let normalized = normalize(&err_string);

            let result: Result<&str, ()> = Ok(normalized.as_str());
            assert_case!(result, $expected);
        }
    };

    // Generic case for unknown field names
    ([$($attrs:meta)*] $field_name:ident, $expected:tt) => {
        compile_error!(concat!("Unknown test case field: ", stringify!($field_name)));
    };

    ([$($attrs:meta)*] error_message, $expected:tt) => {
        $(#[$attrs])*
        #[test]
        fn validate_error_message() {
            let arena = bumpalo::Bump::new();
            let ast = melbi_core::parser::parse(&arena, input()).unwrap();
            let type_manager = melbi_core::types::manager::TypeManager::new();
            let result = melbi_core::analyzer::typed_expr::analyze(&type_manager, &arena, ast);

            assert!(result.is_err(), "Expected type error");
            let err = result.unwrap_err();
            let err_string = format!("{:?}", err);
            assert_case!(Ok(err_string.as_str()), $expected);
        }
    };

    // Generic case for unknown field names
    ([$($attrs:meta)*] $field_name:ident, $expected:tt) => {
        compile_error!(concat!("Unknown test case field: ", stringify!($field_name)));
    };
}

// Helper macro to recursively parse assertion fields (now correctly capturing braces)
#[macro_export]
macro_rules! parse_assertions {
    // Base case: no more fields to parse
    (@parse [$($test_functions:tt)*]) => {
        $($test_functions)*
    };

    // Parse assertion field with potential attributes (handles both cases)
    (@parse [$($test_functions:tt)*] $(#[$attr:meta])* $field_name:ident: $field_value:tt, $($rest:tt)*) => {
        parse_assertions! {@parse [
            $($test_functions)*
            handle_case! {[$($attr)*] $field_name, $field_value}
        ] $($rest)*}
    };
}

// Main macro - name first, input second, then any order for assertions
#[macro_export]
macro_rules! test_case {
    (
        name: $name:ident,
        input: $input:expr,
        $($assertion_fields:tt)*
    ) => {
        mod $name {
            #![allow(unused_imports, dead_code)]

            use super::*;
            use once_cell::sync::OnceCell;

            // Make `$input` available to all test functions
            fn input() -> &'static str {
                static INPUT_CELL: OnceCell<&'static str> = OnceCell::new();
                INPUT_CELL.get_or_init(|| $input)
            }

            // Generate all test functions
            parse_assertions! {@parse [] $($assertion_fields)*}
        }
    };
}
