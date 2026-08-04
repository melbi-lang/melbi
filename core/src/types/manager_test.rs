use alloc::string::ToString;

use bumpalo::Bump;

use super::manager::TypeManager;

#[test]
fn interning() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let float_type = manager.float();

    assert!(core::ptr::eq(int_type, manager.int()));
    assert!(core::ptr::eq(float_type, manager.float()));
}

#[test]
fn interning_record() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let fields = vec![("x", manager.int()), ("y", manager.float())];
    let record_type = manager.record(fields);

    let fields2 = vec![("x", manager.int()), ("y", manager.float())];
    let same_record_type = manager.record(fields2);
    assert!(core::ptr::eq(record_type, same_record_type));

    let fields_unordered = vec![("y", manager.float()), ("x", manager.int())];
    let record_type_unordered = manager.record(fields_unordered);
    assert!(core::ptr::eq(record_type, record_type_unordered));
}

#[test]
fn interning_primitives() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Test Bool
    let bool_type = manager.bool();
    assert!(core::ptr::eq(bool_type, manager.bool()));

    // Test Str
    let str_type = manager.str();
    assert!(core::ptr::eq(str_type, manager.str()));

    // Test Bytes
    let bytes_type = manager.bytes();
    assert!(core::ptr::eq(bytes_type, manager.bytes()));
}

#[test]
fn interning_array() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let int_array = manager.array(int_type);
    let same_int_array = manager.array(int_type);
    assert!(core::ptr::eq(int_array, same_int_array));

    let float_type = manager.float();
    let float_array = manager.array(float_type);
    assert!(!core::ptr::eq(int_array, float_array));

    // Nested arrays
    let nested_array = manager.array(int_array);
    let int_array_again = manager.array(int_type);
    let same_nested_array = manager.array(int_array_again);
    assert!(core::ptr::eq(nested_array, same_nested_array));
}

#[test]
fn interning_map() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let str_to_int = manager.map(str_type, int_type);
    let same_str_to_int = manager.map(str_type, int_type);
    assert!(core::ptr::eq(str_to_int, same_str_to_int));

    let float_type = manager.float();
    let str_to_float = manager.map(str_type, float_type);
    assert!(!core::ptr::eq(str_to_int, str_to_float));

    // Different key types
    let int_to_str = manager.map(int_type, str_type);
    assert!(!core::ptr::eq(str_to_int, int_to_str));
}

#[test]
fn interning_function() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Simple function: (Int) => Bool
    let int_type = manager.int();
    let bool_type = manager.bool();
    let func1 = manager.function(&[int_type], bool_type);
    let same_func1 = manager.function(&[int_type], bool_type);
    assert!(core::ptr::eq(func1, same_func1));

    // Different return type
    let float_type = manager.float();
    let func2 = manager.function(&[int_type], float_type);
    assert!(!core::ptr::eq(func1, func2));

    // Multiple parameters: (Int, Float) => Str
    let str_type = manager.str();
    let func3 = manager.function(&[int_type, float_type], str_type);
    let same_func3 = manager.function(&[int_type, float_type], str_type);
    assert!(core::ptr::eq(func3, same_func3));

    // Different parameter order
    let func4 = manager.function(&[float_type, int_type], str_type);
    assert!(!core::ptr::eq(func3, func4));

    // No parameters: () => Bool
    let func5 = manager.function(&[], bool_type);
    let same_func5 = manager.function(&[], bool_type);
    assert!(core::ptr::eq(func5, same_func5));
}

#[test]
fn interning_symbol() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let sym1 = manager.symbol(vec!["red", "green", "blue"]);
    let same_sym1 = manager.symbol(vec!["red", "green", "blue"]);
    assert!(core::ptr::eq(sym1, same_sym1));

    // Different order should still be equal (symbols are sorted)
    let sym1_unordered = manager.symbol(vec!["blue", "red", "green"]);
    assert!(core::ptr::eq(sym1, sym1_unordered));

    // Different symbols
    let sym2 = manager.symbol(vec!["red", "green"]);
    assert!(!core::ptr::eq(sym1, sym2));

    // Single element symbol
    let sym3 = manager.symbol(vec!["single"]);
    let same_sym3 = manager.symbol(vec!["single"]);
    assert!(core::ptr::eq(sym3, same_sym3));
}

#[test]
fn interning_complex_types() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Array of Records
    let str_type = manager.str();
    let int_type = manager.int();
    let person_record = manager.record(vec![("name", str_type), ("age", int_type)]);
    let people_array = manager.array(person_record);
    let same_person_record = manager.record(vec![("name", str_type), ("age", int_type)]);
    let same_people_array = manager.array(same_person_record);
    assert!(core::ptr::eq(people_array, same_people_array));

    // Function returning Map
    let float_type = manager.float();
    let str_int_map = manager.map(str_type, int_type);
    let func = manager.function(&[str_type], str_int_map);
    let same_str_int_map = manager.map(str_type, int_type);
    let same_func = manager.function(&[str_type], same_str_int_map);
    assert!(core::ptr::eq(func, same_func));

    // Record with complex field types
    let int_array = manager.array(int_type);
    let str_float_map = manager.map(str_type, float_type);
    let complex_record = manager.record(vec![("data", int_array), ("lookup", str_float_map)]);
    let same_int_array = manager.array(int_type);
    let same_str_float_map = manager.map(str_type, float_type);
    let same_complex_record = manager.record(vec![
        ("lookup", same_str_float_map),
        ("data", same_int_array),
    ]);
    assert!(core::ptr::eq(complex_record, same_complex_record));
}

#[test]
fn display_primitives() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    assert_eq!(manager.int().to_string(), "Int");
    assert_eq!(manager.float().to_string(), "Float");
    assert_eq!(manager.bool().to_string(), "Bool");
    assert_eq!(manager.str().to_string(), "Str");
    assert_eq!(manager.bytes().to_string(), "Bytes");
}

#[test]
fn display_array() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let int_array = manager.array(int_type);
    assert_eq!(int_array.to_string(), "Array[Int]");

    let str_type = manager.str();
    let str_array = manager.array(str_type);
    assert_eq!(str_array.to_string(), "Array[Str]");

    // Nested array
    let nested_array = manager.array(int_array);
    assert_eq!(nested_array.to_string(), "Array[Array[Int]]");
}

#[test]
fn display_map() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let str_to_int = manager.map(str_type, int_type);
    assert_eq!(str_to_int.to_string(), "Map[Str, Int]");

    let float_type = manager.float();
    let int_to_float = manager.map(int_type, float_type);
    assert_eq!(int_to_float.to_string(), "Map[Int, Float]");

    // Map with complex value type
    let int_array = manager.array(int_type);
    let str_to_int_array = manager.map(str_type, int_array);
    assert_eq!(str_to_int_array.to_string(), "Map[Str, Array[Int]]");
}

#[test]
fn display_record() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();

    // Simple record
    let person = manager.record(vec![("name", str_type), ("age", int_type)]);
    assert_eq!(person.to_string(), "Record[age: Int, name: Str]");

    // Single field record
    let single = manager.record(vec![("id", int_type)]);
    assert_eq!(single.to_string(), "Record[id: Int]");

    // Record with complex fields
    let int_array = manager.array(int_type);
    let float_type = manager.float();
    let str_float_map = manager.map(str_type, float_type);
    let complex = manager.record(vec![("data", int_array), ("lookup", str_float_map)]);
    assert_eq!(
        complex.to_string(),
        "Record[data: Array[Int], lookup: Map[Str, Float]]"
    );
}

#[test]
fn display_function() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let bool_type = manager.bool();
    let str_type = manager.str();
    let float_type = manager.float();

    // Single parameter function
    let func1 = manager.function(&[int_type], bool_type);
    assert_eq!(func1.to_string(), "(Int) => Bool");

    // Multiple parameters function
    let func2 = manager.function(&[int_type, str_type, float_type], bool_type);
    assert_eq!(func2.to_string(), "(Int, Str, Float) => Bool");

    // No parameters function
    let func3 = manager.function(&[], int_type);
    assert_eq!(func3.to_string(), "() => Int");

    // Function returning function
    let func4 = manager.function(&[int_type], func1);
    assert_eq!(func4.to_string(), "(Int) => (Int) => Bool");

    // Function with complex return type
    let int_array = manager.array(int_type);
    let func5 = manager.function(&[str_type], int_array);
    assert_eq!(func5.to_string(), "(Str) => Array[Int]");
}

#[test]
fn display_symbol() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Multiple symbol parts
    let sym1 = manager.symbol(vec!["red", "green", "blue"]);
    assert_eq!(sym1.to_string(), "Symbol[blue|green|red]");

    // Single symbol part
    let sym2 = manager.symbol(vec!["single"]);
    assert_eq!(sym2.to_string(), "Symbol[single]");

    // Verify symbols are sorted
    let sym3 = manager.symbol(vec!["zebra", "apple", "monkey"]);
    assert_eq!(sym3.to_string(), "Symbol[apple|monkey|zebra]");
}

#[test]
fn display_complex_types() {
    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let float_type = manager.float();

    // Array of Records
    let person_record = manager.record(vec![("name", str_type), ("age", int_type)]);
    let people_array = manager.array(person_record);
    assert_eq!(
        people_array.to_string(),
        "Array[Record[age: Int, name: Str]]"
    );

    // Function returning Map
    let str_int_map = manager.map(str_type, int_type);
    let func = manager.function(&[str_type], str_int_map);
    assert_eq!(func.to_string(), "(Str) => Map[Str, Int]");

    // Record with nested complex types
    let int_array = manager.array(int_type);
    let str_float_map = manager.map(str_type, float_type);
    let func_type = manager.function(&[int_type], str_type);
    let complex_record = manager.record(vec![
        ("data", int_array),
        ("lookup", str_float_map),
        ("transform", func_type),
    ]);
    assert_eq!(
        complex_record.to_string(),
        "Record[data: Array[Int], lookup: Map[Str, Float], transform: (Int) => Str]"
    );

    // Map of Functions
    let bool_type = manager.bool();
    let int_to_bool = manager.function(&[int_type], bool_type);
    let str_to_func = manager.map(str_type, int_to_bool);
    assert_eq!(str_to_func.to_string(), "Map[Str, (Int) => Bool]");
}

#[test]
fn record_with_dynamic_strings() {
    use alloc::string::String;

    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Create field names dynamically (simulating real-world usage like parsing JSON)
    let field1_name = String::from("name");
    let field2_name = "age".to_string();

    // Create record with dynamic strings
    let record1 = manager.record(vec![
        (field1_name.as_str(), manager.str()),
        (field2_name.as_str(), manager.int()),
    ]);

    // Create same record again with fresh dynamic strings
    let field1_name_2 = String::from("name");
    let field2_name_2 = "age".to_string();

    let record2 = manager.record(vec![
        (field1_name_2.as_str(), manager.str()),
        (field2_name_2.as_str(), manager.int()),
    ]);

    // Should be interned to the same pointer
    assert!(
        core::ptr::eq(record1, record2),
        "Records with dynamically generated but equal field names should be interned to the same type"
    );
}

#[test]
fn symbol_with_dynamic_strings() {
    use alloc::string::String;

    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Create symbol parts dynamically
    let part1 = String::from("success");
    let part2 = "error".to_string();
    let part3 = String::from("pending");

    let symbol1 = manager.symbol(vec![part1.as_str(), part2.as_str(), part3.as_str()]);

    // Create same symbol with fresh dynamic strings
    let part1_2 = String::from("success");
    let part2_2 = "error".to_string();
    let part3_2 = String::from("pending");

    let symbol2 = manager.symbol(vec![part1_2.as_str(), part2_2.as_str(), part3_2.as_str()]);

    // Should be interned to the same pointer
    assert!(
        core::ptr::eq(symbol1, symbol2),
        "Symbols with dynamically generated but equal parts should be interned to the same type"
    );
}

#[test]
fn symbol_with_strings_in_vec() {
    use alloc::format;
    use alloc::string::String;

    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Exactly like serialization.rs SymbolPartsSeed::deserialize (line 362-364)
    // Create Vec<String>, then immediately convert to Vec<&str> and call symbol()
    let parts: crate::Vec<crate::String> = vec![
        String::from("success"),
        format!("error"),
        String::from("pending"),
    ];
    let parts_ref: crate::Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    let symbol1 = manager.symbol(parts_ref);

    // Do it again with fresh Strings
    let parts_2: crate::Vec<crate::String> = vec![
        String::from("success"),
        format!("error"),
        String::from("pending"),
    ];
    let parts_ref_2: crate::Vec<&str> = parts_2.iter().map(|s| s.as_str()).collect();
    let symbol2 = manager.symbol(parts_ref_2);

    // Should be interned to the same pointer
    assert!(
        core::ptr::eq(symbol1, symbol2),
        "Symbols created from String vecs should intern to the same type"
    );
}

#[test]
fn record_with_strings_in_vec() {
    use alloc::string::String;

    use crate::types::Type;

    let bump = Bump::new();
    let manager = TypeManager::new(&bump);

    // Simulate deserializing record fields like in serialization.rs RecordFieldsVisitor
    // We receive (String, Type) pairs and need to intern the strings
    let field_data: crate::Vec<(crate::String, &Type)> = vec![
        (String::from("name"), manager.str()),
        ("age".to_string(), manager.int()),
    ];

    // Build Vec<(&str, &Type)> by interning strings (like line 206 in serialization.rs)
    let mut fields: crate::Vec<(&str, &Type)> = crate::Vec::new();
    for (s, t) in &field_data {
        fields.push((manager.intern_str(s.as_str()), *t));
    }
    let record1 = manager.record(fields);

    // Do it again with fresh Strings
    let field_data_2: crate::Vec<(crate::String, &Type)> = vec![
        (String::from("name"), manager.str()),
        ("age".to_string(), manager.int()),
    ];
    let mut fields_2: crate::Vec<(&str, &Type)> = crate::Vec::new();
    for (s, t) in &field_data_2 {
        fields_2.push((manager.intern_str(s.as_str()), *t));
    }
    let record2 = manager.record(fields_2);

    // Should be interned to the same pointer
    assert!(
        core::ptr::eq(record1, record2),
        "Records created from String vecs should intern to the same type"
    );
}
