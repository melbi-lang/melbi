use postcard::to_allocvec;
use serde::Deserialize as _;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, VariantAccess, Visitor};

use crate::types::Type;
use crate::types::manager::TypeManager;
use crate::{Vec, format};

impl<'de, 's, 'a> DeserializeSeed<'de> for &'s TypeManager<'a>
where
    's: 'a,
{
    type Value = &'a Type<'a>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Type", &[], TypeVisitor { mgr: self })
    }
}

struct TypeVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for TypeVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = &'a Type<'a>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a Type enum")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        use serde::de::Error;

        // Read the variant discriminant
        let (discriminant, variant) = data.variant::<u32>()?;

        match discriminant {
            0 => {
                // TypeVar(u16)
                let id = variant.newtype_variant_seed(U16Seed)?;
                Ok(self.mgr.type_var(id))
            }
            1 => {
                // Int
                variant.unit_variant()?;
                Ok(self.mgr.int())
            }
            2 => {
                // Float
                variant.unit_variant()?;
                Ok(self.mgr.float())
            }
            3 => {
                // Bool
                variant.unit_variant()?;
                Ok(self.mgr.bool())
            }
            4 => {
                // Str
                variant.unit_variant()?;
                Ok(self.mgr.str())
            }
            5 => {
                // Bytes
                variant.unit_variant()?;
                Ok(self.mgr.bytes())
            }
            6 => {
                // Array(&'a Type<'a>)
                let inner = variant.newtype_variant_seed(self.mgr)?;
                Ok(self.mgr.array(inner))
            }
            7 => {
                // Map(&'a Type<'a>, &'a Type<'a>)
                let (key, val) = variant.tuple_variant(2, MapVisitor { mgr: self.mgr })?;
                Ok(self.mgr.map(key, val))
            }
            8 => {
                // Record(&'a [(&'a str, &'a Type<'a>)])
                variant.newtype_variant_seed(RecordFieldsSeed { mgr: self.mgr })
            }
            9 => {
                // Function { params: &'a [&'a Type<'a>], ret: &'a Type<'a> }
                let (params, ret) = variant
                    .struct_variant(&["params", "ret"], FunctionVisitor { mgr: self.mgr })?;
                Ok(self.mgr.function(params, ret))
            }
            10 => {
                // Symbol(&'a [&'a str])
                variant.newtype_variant_seed(SymbolPartsSeed { mgr: self.mgr })
            }
            _ => Err(Error::custom(format!(
                "unknown Type variant: {discriminant}"
            ))),
        }
    }
}

// Helper visitor for Map's two type arguments
struct MapVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for MapVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = (&'a Type<'a>, &'a Type<'a>);

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a tuple of two types")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let key = seq
            .next_element_seed(self.mgr)?
            .ok_or_else(|| Error::custom("expected key type"))?;
        let val = seq
            .next_element_seed(self.mgr)?
            .ok_or_else(|| Error::custom("expected value type"))?;
        Ok((key, val))
    }
}

// Helper for deserializing strings (just returns borrowed string from deserializer)
struct StrSeed;

impl<'de> DeserializeSeed<'de> for StrSeed {
    type Value = &'de str;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        <&str>::deserialize(deserializer)
    }
}

// Helper for deserializing u16
struct U16Seed;

impl<'de> DeserializeSeed<'de> for U16Seed {
    type Value = u16;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        u16::deserialize(deserializer)
    }
}

// Helper for deserializing Record fields: &'a [(&'a str, &'a Type<'a>)]
struct RecordFieldsSeed<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> DeserializeSeed<'de> for RecordFieldsSeed<'s, 'a>
where
    's: 'a,
{
    type Value = &'a Type<'a>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(RecordFieldsVisitor { mgr: self.mgr })
    }
}

struct RecordFieldsVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for RecordFieldsVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = &'a Type<'a>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a sequence of record fields")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use crate::Vec;
        // Collect all (&str, Type) pairs
        let mut fields: Vec<(&str, &'a Type<'a>)> = Vec::new();
        while let Some((s, t)) = seq.next_element_seed(RecordFieldSeed { mgr: self.mgr })? {
            fields.push((s, t));
        }

        let result = self.mgr.record(&fields);
        Ok(result)
    }
}

struct RecordFieldSeed<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> DeserializeSeed<'de> for RecordFieldSeed<'s, 'a>
where
    's: 'a,
{
    type Value = (&'de str, &'a Type<'a>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, RecordFieldVisitor { mgr: self.mgr })
    }
}

struct RecordFieldVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for RecordFieldVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = (&'de str, &'a Type<'a>);

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a tuple of (string, type)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let name = seq
            .next_element_seed(StrSeed)?
            .ok_or_else(|| Error::custom("expected field name"))?;
        let ty = seq
            .next_element_seed(self.mgr)?
            .ok_or_else(|| Error::custom("expected field type"))?;
        Ok((name, ty))
    }
}

// Helper for deserializing Function
struct FunctionVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for FunctionVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = (&'a [&'a Type<'a>], &'a Type<'a>);

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a function with params and ret")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;

        use crate::Vec;

        // Deserialize params as Vec
        let params_vec: Vec<&'a Type<'a>> = seq
            .next_element_seed(TypeSliceSeed { mgr: self.mgr })?
            .ok_or_else(|| Error::custom("expected params"))?;
        let ret = seq
            .next_element_seed(self.mgr)?
            .ok_or_else(|| Error::custom("expected return type"))?;

        // Now pass Vec to function() which will allocate the slice
        let func = self.mgr.function(&params_vec, ret);
        if let Type::Function { params, ret } = func {
            Ok((params, ret))
        } else {
            unreachable!()
        }
    }
}

// Helper for deserializing Vec<&'a Type<'a>>
struct TypeSliceSeed<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> DeserializeSeed<'de> for TypeSliceSeed<'s, 'a>
where
    's: 'a,
{
    type Value = crate::Vec<&'a Type<'a>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(TypeSliceVisitor { mgr: self.mgr })
    }
}

struct TypeSliceVisitor<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> Visitor<'de> for TypeSliceVisitor<'s, 'a>
where
    's: 'a,
{
    type Value = crate::Vec<&'a Type<'a>>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a sequence of types")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use crate::Vec;
        let mut types: Vec<&'a Type<'a>> = Vec::new();
        while let Some(ty) = seq.next_element_seed(self.mgr)? {
            types.push(ty);
        }
        Ok(types)
    }
}

// Helper for deserializing Symbol parts as Vec<String>
struct SymbolPartsSeed<'s, 'a> {
    mgr: &'s TypeManager<'a>,
}

impl<'de, 's, 'a> DeserializeSeed<'de> for SymbolPartsSeed<'s, 'a>
where
    's: 'a,
{
    type Value = &'a Type<'a>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parts = deserializer.deserialize_seq(SymbolPartsVisitor)?;
        Ok(self.mgr.symbol(&parts))
    }
}

struct SymbolPartsVisitor;

impl<'de> Visitor<'de> for SymbolPartsVisitor {
    type Value = crate::Vec<&'de str>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a sequence of strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use crate::Vec;
        let mut parts: Vec<&'de str> = Vec::new();
        while let Some(part) = seq.next_element_seed(StrSeed)? {
            parts.push(part);
        }
        Ok(parts)
    }
}

impl<'a> TypeManager<'a> {
    pub fn serialize_type(&self, ty: &Type<'a>) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(ty)
    }

    pub fn deserialize_type<'s>(&'s self, bytes: &[u8]) -> Result<&'a Type<'a>, postcard::Error>
    where
        's: 'a,
    {
        let mut deserializer = postcard::Deserializer::from_bytes(bytes);
        use serde::de::DeserializeSeed;
        self.deserialize(&mut deserializer)
    }
}
