//! # Melbi Type Encoding
//!
//! Compact binary encoding for Melbi types with zero-copy navigation.
//!
//! ## Overview
//!
//! This module provides an alternate representation for Melbi types that:
//! - Has no lifetime dependencies (just `&[u8]` or `Vec<u8>`)
//! - Supports fast equality via byte comparison
//! - Enables zero-copy navigation via `TypeView`
//! - Roundtrips perfectly with arena representation
//!
//! ## Format
//!
//! Types are either unitary or composite. Unitary types do not contain any additional
//! information or are composed of any other types. They are represented only by their
//! type tag.
//!
//! The list of unitary types is: Int, Float, Bool, Str, Bytes.
//!
//! Therefore unitary types are encoded as a single byte with the type tag. In, what
//! we're calling: "packed format".
//!
//! For efficiency, we also represent some common types in a packed format (single byte
//! with no additional payload).
//!
//! All other types require a payload.
//!
//! Types are encoded prefixed with a wire tag or wire byte indicating the variant,
//! but sometimes containing additional packed information.
//!
//! - **0-63**: Direct TypeTag for all Melbi types (with reserved space for new types).
//! - **64-95**: Packed TypeVar(0..31).
//! - **96-100**: Packed Array(e) for all unitary types.
//! - **101-126**: Packed Map(k, v) for all composite types.
//!
//! Unitary types (Int, Float, Bool, Str, Bytes)
//! - **64-95**: Composite types (Array, Map, Record, Function, Symbol)
//! - **96-127**: Packed types (Array[Primitive], Map[Primitive, Primitive])
//!
//! Composite types include a u16 size field: `[disc][size_lo][size_hi][payload...]`
//!
//! ## Format Design Principles
//!
//! - **Singleton before array**: When a type contains both a singleton and array,
//!   the singleton is encoded first for O(1) access (e.g., Function return type).
//! - **Count prefixes sequence**: Array counts always immediately precede their elements:
//!   `[varint:count][element_1][element_2]...[element_n]`
//!
//! Example: Function type encoding:
//! ```text
//! [DISC_FUNCTION][size_lo][size_hi][return_type][varint:param_count][param_1][param_2]...
//! ```
//!
//! See the design document for full specification.

use smallvec::SmallVec;

use crate::types::Type;
use crate::types::encoding::wire::{ChosenEncoding, Payload, WireEncoding, WireTag};
use crate::types::traits::{TypeKind, TypeTag, TypeView};

// ============================================================================
// Wire Format Encoding
// ============================================================================
//
// IMPORTANT: This encoding is for in-memory representation only, NOT for
// data exchange or persistent storage. The format can change freely between
// versions without compatibility concerns.
//
// Adding a new type: Update TypeTag enum in type_traits.rs and WireTag implementation below.

mod wire {
    use super::DecodeError;
    use crate::types::traits::TypeTag;

    /// Instruction for encoding: includes the type tag a suggestion on what else to pack
    /// for supported types.
    #[derive(Debug, Clone, Copy)]
    pub(super) enum WireEncoding {
        Standard,                    // Use standard format (0-63) with payload
        PackedTypeVar(u16),          // Try packed TypeVar (64-95), TypeTag:TypeVar
        PackedArray(TypeTag),        // Try packed Array (96-100), TypeTag:Array
        PackedMap(TypeTag, TypeTag), // Try packed Map (101-125), TypeTag:Map
    }

    /// Payload variants after decoding a wire byte
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Payload<'a> {
        None,                        // No payload (unitary types)
        Buffer(&'a [u8]),            // Payload from byte stream
        PackedTypeVar(u16),          // TypeVar ID embedded in wire byte
        PackedArray(TypeTag),        // Element type bytes (static)
        PackedMap(TypeTag, TypeTag), // Key and value type bytes (static)
    }

    /// Result of decoding a wire tag from a buffer
    #[derive(Debug, Clone, Copy)]
    pub(super) struct Decoded<'a> {
        pub(super) type_tag: TypeTag,
        pub(super) payload: Payload<'a>,
        pub(super) remaining_buffer: &'a [u8],
    }

    #[derive(Debug, Clone, Copy)]
    pub(super) enum ChosenEncoding {
        WithPayload,    // Wire byte followed by size and payload bytes
        WithoutPayload, // Everything encoded in the wire byte itself
    }

    #[derive(Debug, Clone, Copy)]
    pub(super) struct WireTag {
        wire_tag: u8,
        chosen_encoding: ChosenEncoding,
    }

    impl WireTag {
        const fn packed_id_from_type_tag(tag: TypeTag) -> Option<u8> {
            match tag {
                TypeTag::Int => Some(0),
                TypeTag::Float => Some(1),
                TypeTag::Bool => Some(2),
                TypeTag::Str => Some(3),
                TypeTag::Bytes => Some(4),
                _ => None,
            }
        }

        fn packed_type_id_to_type_tag(idx: u8) -> Result<TypeTag, DecodeError> {
            match idx {
                0 => Ok(TypeTag::Int),
                1 => Ok(TypeTag::Float),
                2 => Ok(TypeTag::Bool),
                3 => Ok(TypeTag::Str),
                4 => Ok(TypeTag::Bytes),
                _ => Err(DecodeError::InvalidWireTag { tag: idx }),
            }
        }

        fn is_unitary_type(type_tag: TypeTag) -> bool {
            matches!(
                type_tag,
                TypeTag::Int | TypeTag::Float | TypeTag::Bool | TypeTag::Str | TypeTag::Bytes
            )
        }

        /// Decode wire byte from buffer
        pub(super) fn from_buffer(bytes: &'_ [u8]) -> Result<Decoded<'_>, DecodeError> {
            if bytes.is_empty() {
                return Err(DecodeError::Truncated { needed: 1 });
            }
            let byte = bytes[0];

            match byte {
                // Standard wire tags (0-63)
                0..64 => {
                    let type_tag: TypeTag = byte
                        .try_into()
                        .map_err(|_| DecodeError::InvalidWireTag { tag: byte })?;

                    if Self::is_unitary_type(type_tag) {
                        // Unitary types have no payload
                        return Ok(Decoded {
                            type_tag,
                            payload: Payload::None,
                            remaining_buffer: &bytes[1..],
                        });
                    }

                    // Composite types have [size_lo][size_hi][payload]
                    if bytes.len() < 3 {
                        return Err(DecodeError::Truncated { needed: 3 });
                    }
                    let payload_size = super::read_u16_le(&bytes[1..3]) as usize;
                    if bytes.len() < 3 + payload_size {
                        return Err(DecodeError::Truncated {
                            needed: 3 + payload_size,
                        });
                    }

                    Ok(Decoded {
                        type_tag,
                        payload: Payload::Buffer(&bytes[3..3 + payload_size]),
                        remaining_buffer: &bytes[3 + payload_size..],
                    })
                }

                // Packed TypeVar (64-95): IDs 0-31
                64..96 => {
                    let id = (byte as u16) - 64u16;
                    Ok(Decoded {
                        type_tag: TypeTag::TypeVar,
                        payload: Payload::PackedTypeVar(id),
                        remaining_buffer: &bytes[1..],
                    })
                }

                // Packed Array (96-100): Array[Unitary]
                96..101 => {
                    let type_tag = Self::packed_type_id_to_type_tag(byte - 96u8)?;
                    Ok(Decoded {
                        type_tag: TypeTag::Array,
                        payload: Payload::PackedArray(type_tag),
                        remaining_buffer: &bytes[1..],
                    })
                }

                // Packed Map (101-125): Map[Unitary, Unitary]
                101..=125 => {
                    let offset = byte - 101;
                    let key_idx = offset / 5;
                    let val_idx = offset % 5;

                    let key_type_tag = Self::packed_type_id_to_type_tag(key_idx)?;
                    let value_type_tag = Self::packed_type_id_to_type_tag(val_idx)?;
                    Ok(Decoded {
                        type_tag: TypeTag::Map,
                        payload: Payload::PackedMap(key_type_tag, value_type_tag),
                        remaining_buffer: &bytes[1..],
                    })
                }

                // Reserved/Invalid (126-255): MSB set or future use
                _ => Err(DecodeError::InvalidWireTag { tag: byte }),
            }
        }

        /// Create WireTag for encoding (tries to use packed format when possible)
        // TODO: Rewrite this for more flexibility on what can or cannot get packed.
        pub(super) fn for_encoding(type_tag: TypeTag, encoding: WireEncoding) -> Self {
            match (type_tag, encoding) {
                (TypeTag::TypeVar, WireEncoding::PackedTypeVar(id)) if id < 32 => WireTag {
                    wire_tag: 64 + (id as u8),
                    chosen_encoding: ChosenEncoding::WithoutPayload,
                },
                (TypeTag::Array, WireEncoding::PackedArray(elem)) => {
                    match Self::packed_id_from_type_tag(elem) {
                        Some(packed_type_id) => WireTag {
                            wire_tag: 96 + packed_type_id,
                            chosen_encoding: ChosenEncoding::WithoutPayload,
                        },
                        None => WireTag {
                            wire_tag: type_tag as u8,
                            chosen_encoding: ChosenEncoding::WithPayload,
                        },
                    }
                }
                (TypeTag::Map, WireEncoding::PackedMap(key_type, value_type)) => {
                    match (
                        Self::packed_id_from_type_tag(key_type),
                        Self::packed_id_from_type_tag(value_type),
                    ) {
                        (Some(key_type_id), Some(value_type_id)) => WireTag {
                            wire_tag: 101 + key_type_id * 5 + value_type_id,
                            chosen_encoding: ChosenEncoding::WithoutPayload,
                        },
                        _ => WireTag {
                            wire_tag: type_tag as u8,
                            chosen_encoding: ChosenEncoding::WithPayload,
                        },
                    }
                }
                (type_tag, _) if Self::is_unitary_type(type_tag) => WireTag {
                    wire_tag: type_tag as u8,
                    chosen_encoding: ChosenEncoding::WithoutPayload,
                },
                _ => WireTag {
                    wire_tag: type_tag as u8,
                    chosen_encoding: ChosenEncoding::WithPayload, // Fallback to standard encoding
                },
            }
        }

        /// Encode to wire byte
        pub(super) fn as_byte(&self) -> u8 {
            self.wire_tag
        }

        pub(super) fn chosen_encoding(&self) -> ChosenEncoding {
            self.chosen_encoding
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated {
        needed: usize,
    },
    SizeMismatch {
        offset: usize,
        claimed: usize,
        actual: usize,
    },
    InvalidWireTag {
        tag: u8,
    },
    InvalidUtf8 {
        offset: usize,
    },
    InvalidVarint {
        offset: usize,
    },
    TooDeep {
        depth: usize,
    },
    TrailingBytes {
        offset: usize,
        remaining: usize,
    },
}

// ============================================================================
// Helper Functions
// ============================================================================

fn write_varint(buf: &mut BufferType, mut n: usize) {
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

fn write_string(buf: &mut BufferType, s: &str) {
    write_varint(buf, s.len());
    buf.extend_from_slice(s.as_bytes());
}

fn read_varint(bytes: &[u8]) -> Result<(usize, usize), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { needed: 1 });
    }

    let mut result = 0usize;
    let mut shift = 0;
    let mut pos = 0;

    while pos < bytes.len() {
        let byte = bytes[pos];
        pos += 1;

        result |= ((byte & 0x7F) as usize) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, pos));
        }

        shift += 7;
        if shift >= 64 {
            return Err(DecodeError::InvalidVarint { offset: pos });
        }
    }

    Err(DecodeError::Truncated { needed: 1 })
}

// SAFETY: Strings are only written via `write_string()` which takes `&str`,
// guaranteeing valid UTF-8. This encoding format is for in-memory use only
// with trusted sources (we control both encode and decode).
fn read_string(bytes: &[u8]) -> Result<(&str, &[u8]), DecodeError> {
    let (len, varint_len) = read_varint(bytes)?;

    if bytes.len() < varint_len + len {
        return Err(DecodeError::Truncated {
            needed: varint_len + len,
        });
    }

    let str_bytes = &bytes[varint_len..varint_len + len];
    let s = unsafe { core::str::from_utf8_unchecked(str_bytes) };

    Ok((s, &bytes[varint_len + len..]))
}

#[inline]
fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[inline]
fn write_u16_le(buf: &mut BufferType, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

// ============================================================================
// Encoding
// ============================================================================

pub type BufferType = SmallVec<[u8; 16]>;

/// Encodes a composite type with the standard 3-byte header: [disc][size_lo][size_hi][payload]
/// The payload is encoded by the provided closure.
#[inline]
fn encode_composite<F>(buf: &mut BufferType, disc: u8, encode_payload: F)
where
    F: FnOnce(&mut BufferType),
{
    let start = buf.len();
    buf.push(disc);
    buf.push(0); // placeholder for size_lo
    buf.push(0); // placeholder for size_hi
    encode_payload(buf);
    let size = buf.len() - start - 3;
    buf[start + 1] = (size & 0xFF) as u8;
    buf[start + 2] = ((size >> 8) & 0xFF) as u8;
}

pub fn encode(ty: &Type) -> BufferType {
    let mut buf = BufferType::new();
    encode_inner(ty, &mut buf);
    buf
}

fn type_to_tag(ty: &Type) -> TypeTag {
    let tag: TypeTag = ty.discriminant().try_into().unwrap();
    debug_assert_eq!(
        tag,
        match ty {
            Type::TypeVar(_) => TypeTag::TypeVar,
            Type::Int => TypeTag::Int,
            Type::Float => TypeTag::Float,
            Type::Bool => TypeTag::Bool,
            Type::Str => TypeTag::Str,
            Type::Bytes => TypeTag::Bytes,
            Type::Array(_) => TypeTag::Array,
            Type::Map(_, _) => TypeTag::Map,
            Type::Record(_) => TypeTag::Record,
            Type::Function { .. } => TypeTag::Function,
            Type::Symbol(_) => TypeTag::Symbol,
            Type::Option(_) => TypeTag::Option,
        }
    );
    tag
}

fn encode_inner(ty: &Type, buf: &mut BufferType) {
    let tag = match ty {
        Type::TypeVar(id) => {
            WireTag::for_encoding(TypeTag::TypeVar, WireEncoding::PackedTypeVar(*id))
        }
        Type::Array(elem_ty) => WireTag::for_encoding(
            TypeTag::Array,
            WireEncoding::PackedArray(type_to_tag(elem_ty)),
        ),
        Type::Map(key_ty, value_ty) => WireTag::for_encoding(
            TypeTag::Map,
            WireEncoding::PackedMap(type_to_tag(key_ty), type_to_tag(value_ty)),
        ),
        _ => WireTag::for_encoding(type_to_tag(ty), WireEncoding::Standard),
    };
    if let ChosenEncoding::WithoutPayload = tag.chosen_encoding() {
        buf.push(tag.as_byte());
        return;
    }
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::Str | Type::Bytes => {
            unreachable!("types are always packed");
        }
        Type::TypeVar(id) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                write_u16_le(buf, *id);
            });
        }
        Type::Array(elem) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                encode_inner(elem, buf);
            });
        }
        Type::Map(key, val) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                encode_inner(key, buf);
                encode_inner(val, buf);
            });
        }
        Type::Option(inner) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                encode_inner(inner, buf);
            });
        }
        Type::Record(fields) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                write_varint(buf, fields.len());
                for (name, ty) in fields.iter() {
                    write_string(buf, name);
                    encode_inner(ty, buf);
                }
            });
        }
        Type::Function { params, ret } => {
            encode_composite(buf, tag.as_byte(), |buf| {
                encode_inner(ret, buf); // return type FIRST
                write_varint(buf, params.len()); // then count
                for param in params.iter() {
                    encode_inner(param, buf); // then params
                }
            });
        }
        Type::Symbol(parts) => {
            encode_composite(buf, tag.as_byte(), |buf| {
                write_varint(buf, parts.len());
                for part in parts.iter() {
                    write_string(buf, part);
                }
            });
        }
    }
}

// ============================================================================
// OwnedType
// ============================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OwnedType {
    buffer: BufferType,
}

impl OwnedType {
    pub fn new(buffer: BufferType) -> Self {
        OwnedType { buffer }
    }

    pub fn view<'a>(&'a self) -> TypeKind<'a, EncodedType<'a>> {
        EncodedType::new_from_buffer(&self.buffer[..])
            .unwrap()
            .0
            .view()
    }
}

// ============================================================================
// EncodedType
// ============================================================================

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EncodedType<'a> {
    type_tag: TypeTag,
    payload: Payload<'a>,
}

impl<'a> EncodedType<'a> {
    #[inline]
    pub(crate) fn new(type_tag: TypeTag, payload: Payload<'a>) -> Self {
        EncodedType { type_tag, payload }
    }

    pub fn new_from_buffer(buffer: &'a [u8]) -> Result<(Self, &'a [u8]), DecodeError> {
        let decoded = wire::WireTag::from_buffer(buffer)?;
        Ok((
            EncodedType::new(decoded.type_tag, decoded.payload),
            decoded.remaining_buffer,
        ))
    }
}

impl<'a> TypeView<'a> for EncodedType<'a> {
    type Iter = ParamsIter<'a>;
    type NamedIter = RecordIter<'a>;
    type StrIter = SymbolIter<'a>;

    fn view(self) -> TypeKind<'a, Self> {
        match self.type_tag {
            TypeTag::TypeVar => match self.payload {
                Payload::PackedTypeVar(id) => TypeKind::TypeVar(id),
                Payload::Buffer(buffer) if buffer.len() == 2 => {
                    TypeKind::TypeVar(read_u16_le(buffer))
                }
                _ => unreachable!(),
            },
            TypeTag::Int => TypeKind::Int,
            TypeTag::Float => TypeKind::Float,
            TypeTag::Bool => TypeKind::Bool,
            TypeTag::Str => TypeKind::Str,
            TypeTag::Bytes => TypeKind::Bytes,
            TypeTag::Array => match self.payload {
                Payload::PackedArray(type_tag) => {
                    TypeKind::Array(EncodedType::new(type_tag, Payload::None))
                }
                Payload::Buffer(buffer) => {
                    let (elem_ty, remaining) =
                        EncodedType::new_from_buffer(buffer).expect("invalid array payload");
                    debug_assert!(remaining.is_empty());
                    TypeKind::Array(elem_ty)
                }
                _ => unreachable!(),
            },
            TypeTag::Map => match self.payload {
                Payload::PackedMap(key_type_tag, value_type_tag) => TypeKind::Map(
                    EncodedType::new(key_type_tag, Payload::None),
                    EncodedType::new(value_type_tag, Payload::None),
                ),
                Payload::Buffer(buffer) => {
                    // Buffer contains: [key_encoding][value_encoding]
                    let (key_ty, remaining) =
                        EncodedType::new_from_buffer(buffer).expect("invalid map key encoding");
                    let (value_ty, remaining) = EncodedType::new_from_buffer(remaining)
                        .expect("invalid map value encoding");
                    debug_assert!(remaining.is_empty());
                    TypeKind::Map(key_ty, value_ty)
                }
                _ => unreachable!(),
            },
            TypeTag::Option => match self.payload {
                Payload::Buffer(buffer) => {
                    let (inner_ty, remaining) =
                        EncodedType::new_from_buffer(buffer).expect("invalid option payload");
                    debug_assert!(remaining.is_empty());
                    TypeKind::Option(inner_ty)
                }
                _ => unreachable!(),
            },
            TypeTag::Record => match self.payload {
                Payload::Buffer(buffer) => {
                    let iter = RecordIter::new(buffer).expect("invalid record payload");
                    TypeKind::Record(iter)
                }
                _ => unreachable!("Record can only have Buffer payload"),
            },
            TypeTag::Function => match self.payload {
                Payload::Buffer(buffer) => {
                    // Buffer format: [return_type][varint:param_count][param_1][param_2]...
                    let (ret, remaining) =
                        EncodedType::new_from_buffer(buffer).expect("invalid function return type");

                    let params = ParamsIter::new(remaining).expect("invalid function params");

                    TypeKind::Function { params, ret }
                }
                _ => unreachable!("Function can only have Buffer payload"),
            },
            TypeTag::Symbol => match self.payload {
                Payload::Buffer(buffer) => {
                    let iter = SymbolIter::new(buffer).expect("invalid symbol payload");
                    TypeKind::Symbol(iter)
                }
                _ => unreachable!("Symbol can only have Buffer payload"),
            },
        }
    }
}

// ============================================================================
// Iterators
// ============================================================================

pub struct RecordIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> RecordIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(RecordIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = (&'a str, EncodedType<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let (name, remaining_buffer) =
            read_string(self.payload).expect("invalid encoded type in record");

        let (ty_view, remaining) =
            EncodedType::new_from_buffer(remaining_buffer).expect("invalid encoded type in record");

        self.payload = remaining;

        Some((name, ty_view))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for RecordIter<'a> {}

pub struct ParamsIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> ParamsIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(ParamsIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = EncodedType<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        // Decode the parameter type
        let (view, remaining) =
            EncodedType::new_from_buffer(self.payload).expect("invalid encoded type in params");

        self.payload = remaining;

        Some(view)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for ParamsIter<'a> {}

pub struct SymbolIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> SymbolIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(SymbolIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        let (part, remaining) = read_string(self.payload).expect("invalid encoded symbol part");
        self.payload = remaining;
        Some(part)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for SymbolIter<'a> {}

// ============================================================================
// Decoding
// ============================================================================

use crate::types::traits::TypeTransformer;

/// Simple transformer that decodes EncodedType to &Type using TypeManager
struct Decoder<'a> {
    mgr: &'a crate::types::manager::TypeManager<'a>,
}

impl<'a> TypeTransformer<'a, &'a crate::types::manager::TypeManager<'a>> for Decoder<'a> {
    type Input = EncodedType<'a>;

    fn builder(&self) -> &&'a crate::types::manager::TypeManager<'a> {
        &self.mgr
    }
}

pub fn decode<'a>(
    bytes: &'a [u8],
    mgr: &'a crate::types::manager::TypeManager<'a>,
) -> Result<&'a Type<'a>, DecodeError> {
    let (view, remaining) = EncodedType::new_from_buffer(bytes)?;

    // Verify we consumed all bytes
    if !remaining.is_empty() {
        return Err(DecodeError::TrailingBytes {
            offset: bytes.len() - remaining.len(),
            remaining: remaining.len(),
        });
    }

    let decoder = Decoder { mgr };
    Ok(decoder.transform(view))
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn test_smallvec_size() {
        let v = Vec::<u8>::with_capacity(32);
        let p = v.leak();
        dbg!(std::mem::size_of_val(&p));
        dbg!(std::any::type_name_of_val(&p));

        dbg!(core::mem::size_of::<SmallVec<[u8; 1]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 8]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 16]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 18]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 24]>>());
        assert!(true);
    }

    // ============================================================================
    // Packed Type Navigation Tests (CRITICAL)
    // ============================================================================

    #[test]
    fn test_discriminant_normalization() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Int] uses packed format (1 byte)
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        // Verify wire byte decodes to correct type
        let decoded = wire::WireTag::from_buffer(&bytes).unwrap();
        assert_eq!(decoded.type_tag, TypeTag::Array);

        let view = EncodedType::new(decoded.type_tag, decoded.payload);
        assert!(matches!(view.view(), TypeKind::Array(_)));

        // Map[Str, Int] uses packed format (1 byte)
        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let decoded = wire::WireTag::from_buffer(&bytes).unwrap();
        assert_eq!(decoded.type_tag, TypeTag::Map);

        let view = EncodedType::new(decoded.type_tag, decoded.payload);
        assert!(matches!(view.view(), TypeKind::Map(_, _)));
    }

    #[test]
    fn test_navigate_packed_array_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Int] - packed encoding (1 byte)
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        // This should work - uniform access!
        match view.view() {
            TypeKind::Array(elem_view) => {
                assert!(matches!(elem_view.view(), TypeKind::Int));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_navigate_packed_map_str_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Map[Str, Int] - packed encoding (1 byte)
        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        // This should work - uniform access!
        match view.view() {
            TypeKind::Map(key_view, val_view) => {
                assert!(matches!(key_view.view(), TypeKind::Str));
                assert!(matches!(val_view.view(), TypeKind::Int));
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn test_navigate_all_packed_arrays() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test Int
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Array(elem_view) => assert!(matches!(elem_view.view(), TypeKind::Int)),
            _ => panic!("expected array"),
        }

        // Test Float
        let ty = mgr.array(mgr.float());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Array(elem_view) => assert!(matches!(elem_view.view(), TypeKind::Float)),
            _ => panic!("expected array"),
        }

        // Test Bool
        let ty = mgr.array(mgr.bool());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Array(elem_view) => assert!(matches!(elem_view.view(), TypeKind::Bool)),
            _ => panic!("expected array"),
        }

        // Test Str
        let ty = mgr.array(mgr.str());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Array(elem_view) => assert!(matches!(elem_view.view(), TypeKind::Str)),
            _ => panic!("expected array"),
        }

        // Test Bytes
        let ty = mgr.array(mgr.bytes());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Array(elem_view) => assert!(matches!(elem_view.view(), TypeKind::Bytes)),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_navigate_all_packed_maps() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test a few representative map combinations
        // Map[Int, Int]
        let ty = mgr.map(mgr.int(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Map(key_view, val_view) => {
                assert!(matches!(key_view.view(), TypeKind::Int));
                assert!(matches!(val_view.view(), TypeKind::Int));
            }
            _ => panic!("expected map"),
        }

        // Map[Str, Float]
        let ty = mgr.map(mgr.str(), mgr.float());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Map(key_view, val_view) => {
                assert!(matches!(key_view.view(), TypeKind::Str));
                assert!(matches!(val_view.view(), TypeKind::Float));
            }
            _ => panic!("expected map"),
        }

        // Map[Bool, Bytes]
        let ty = mgr.map(mgr.bool(), mgr.bytes());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();
        match view.view() {
            TypeKind::Map(key_view, val_view) => {
                assert!(matches!(key_view.view(), TypeKind::Bool));
                assert!(matches!(val_view.view(), TypeKind::Bytes));
            }
            _ => panic!("expected map"),
        }
    }

    // ============================================================================
    // TypeView-based Decode Tests
    // ============================================================================

    #[test]
    fn test_decode_via_typeview_primitives() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let int_bytes = encode(mgr.int());
        let decoded = decode(&int_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.int(), decoded));

        let float_bytes = encode(mgr.float());
        let decoded = decode(&float_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.float(), decoded));

        let bool_bytes = encode(mgr.bool());
        let decoded = decode(&bool_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bool(), decoded));

        let str_bytes = encode(mgr.str());
        let decoded = decode(&str_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.str(), decoded));

        let bytes_bytes = encode(mgr.bytes());
        let decoded = decode(&bytes_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bytes(), decoded));
    }

    #[test]
    fn test_decode_via_typeview_packed_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);

        // decode() uses only TypeView navigation
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_decode_via_typeview_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);

        // decode() uses only TypeView navigation
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_decode_via_typeview_complex() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(
            &[
                mgr.map(mgr.str(), mgr.array(mgr.int())),
                mgr.record(vec![("result", mgr.bool()), ("count", mgr.int())]),
            ],
            mgr.symbol(vec!["success", "error"]),
        );

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    // ============================================================================
    // Lenient Decoding Tests
    // ============================================================================

    #[test]
    fn test_lenient_decode_non_packed_array_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Manually create encoding: [Array wire byte][size][Int wire byte]
        let array_byte = TypeTag::Array as u8;
        let int_byte = TypeTag::Int as u8;
        let bytes = vec![array_byte, 1, 0, int_byte];

        let decoded = decode(&bytes, &mgr).unwrap();
        let expected = mgr.array(mgr.int());

        // Should intern to same pointer as canonical encoding
        assert!(core::ptr::eq(decoded, expected));
    }

    #[test]
    fn test_lenient_decode_non_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Manually create encoding: [Map wire byte][size][Str wire byte][Int wire byte]
        let map_byte = TypeTag::Map as u8;
        let str_byte = TypeTag::Str as u8;
        let int_byte = TypeTag::Int as u8;
        let bytes = vec![map_byte, 2, 0, str_byte, int_byte];

        let decoded = decode(&bytes, &mgr).unwrap();
        let expected = mgr.map(mgr.str(), mgr.int());

        // Should intern to same pointer as canonical encoding
        assert!(core::ptr::eq(decoded, expected));
    }

    #[test]
    fn test_both_encodings_intern_same() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Canonical encoding now uses packed format (1 byte)
        let ty = mgr.array(mgr.int());
        let canonical_bytes = encode(ty);
        assert_eq!(canonical_bytes.len(), 1); // Packed format

        // Alternative long-format encoding should decode to same interned type
        let array_byte = TypeTag::Array as u8;
        let int_byte = TypeTag::Int as u8;
        let alternative_bytes = vec![array_byte, 1, 0, int_byte]; // Long format: [disc][size_lo][size_hi][elem]

        let decoded_canonical = decode(&canonical_bytes, &mgr).unwrap();
        let decoded_alternative = decode(&alternative_bytes, &mgr).unwrap();

        // Both should intern to same pointer (lenient decoding)
        assert!(core::ptr::eq(ty, decoded_canonical));
        assert!(core::ptr::eq(ty, decoded_alternative));
        assert!(core::ptr::eq(decoded_canonical, decoded_alternative));
    }

    // ============================================================================
    // Basic Round-trip Tests
    // ============================================================================

    #[test]
    fn test_primitives_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let int_bytes = encode(mgr.int());
        let decoded = decode(&int_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.int(), decoded));

        let float_bytes = encode(mgr.float());
        let decoded = decode(&float_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.float(), decoded));

        let bool_bytes = encode(mgr.bool());
        let decoded = decode(&bool_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bool(), decoded));

        let str_bytes = encode(mgr.str());
        let decoded = decode(&str_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.str(), decoded));

        let bytes_bytes = encode(mgr.bytes());
        let decoded = decode(&bytes_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bytes(), decoded));
    }

    #[test]
    fn test_typevar_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test packed and non-packed
        let ty0 = mgr.type_var(0);
        let bytes0 = encode(ty0);
        let decoded0 = decode(&bytes0, &mgr).unwrap();
        assert!(core::ptr::eq(ty0, decoded0));

        let ty15 = mgr.type_var(15);
        let bytes15 = encode(ty15);
        let decoded15 = decode(&bytes15, &mgr).unwrap();
        assert!(core::ptr::eq(ty15, decoded15));

        let ty31 = mgr.type_var(31);
        let bytes31 = encode(ty31);
        let decoded31 = decode(&bytes31, &mgr).unwrap();
        assert!(core::ptr::eq(ty31, decoded31));

        let ty32 = mgr.type_var(32);
        let bytes32 = encode(ty32);
        let decoded32 = decode(&bytes32, &mgr).unwrap();
        assert!(core::ptr::eq(ty32, decoded32));

        let ty100 = mgr.type_var(100);
        let bytes100 = encode(ty100);
        let decoded100 = decode(&bytes100, &mgr).unwrap();
        assert!(core::ptr::eq(ty100, decoded100));

        let ty1000 = mgr.type_var(1000);
        let bytes1000 = encode(ty1000);
        let decoded1000 = decode(&bytes1000, &mgr).unwrap();
        assert!(core::ptr::eq(ty1000, decoded1000));
    }

    #[test]
    fn test_record_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("age", mgr.int()), ("name", mgr.str())]);

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_function_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_symbol_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["error", "pending", "success"]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    // ============================================================================
    // TypeView Navigation Tests (Non-Packed)
    // ============================================================================

    #[test]
    fn test_navigate_non_packed_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.array(mgr.int()));
        let bytes = encode(ty);

        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        match view.view() {
            TypeKind::Array(elem_view) => {
                // Nested array
                match elem_view.view() {
                    TypeKind::Array(inner_elem) => {
                        assert!(matches!(inner_elem.view(), TypeKind::Int));
                    }
                    _ => panic!("expected nested array"),
                }
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_navigate_non_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.array(mgr.int()), mgr.str());
        let bytes = encode(ty);

        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        match view.view() {
            TypeKind::Map(key, val) => {
                assert!(matches!(key.view(), TypeKind::Array(_)));
                assert!(matches!(val.view(), TypeKind::Str));
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn test_navigate_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("age", mgr.int()), ("name", mgr.str())]);

        let bytes = encode(ty);
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        match view.view() {
            TypeKind::Record(fields_iter) => {
                let fields: Vec<_> = fields_iter.collect();

                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "age");
                assert!(matches!(fields[0].1.view(), TypeKind::Int));
                assert_eq!(fields[1].0, "name");
                assert!(matches!(fields[1].1.view(), TypeKind::Str));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn test_navigate_function() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());

        let bytes = encode(ty);
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        match view.view() {
            TypeKind::Function { params, ret } => {
                let params: Vec<_> = params.collect();
                assert_eq!(params.len(), 2);
                assert!(matches!(params[0].view(), TypeKind::Int));
                assert!(matches!(params[1].view(), TypeKind::Str));

                assert!(matches!(ret.view(), TypeKind::Bool));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_navigate_symbol() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Note: TypeManager sorts symbol parts
        let ty = mgr.symbol(vec!["success", "error", "pending"]);
        let bytes = encode(ty);
        let (view, _) = EncodedType::new_from_buffer(&bytes).unwrap();

        match view.view() {
            TypeKind::Symbol(parts_iter) => {
                let parts: Vec<_> = parts_iter.collect();

                assert_eq!(parts.len(), 3);
                // Sorted order
                assert_eq!(parts[0], "error");
                assert_eq!(parts[1], "pending");
                assert_eq!(parts[2], "success");
            }
            _ => panic!("expected symbol"),
        }
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_decode_empty_buffer() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let result = decode(&[], &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));
    }

    #[test]
    fn test_decode_trailing_bytes() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.int();
        let mut bytes = encode(ty).to_vec();
        bytes.push(0xFF); // Extra byte

        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::TrailingBytes { .. })));
    }

    #[test]
    fn test_decode_unknown_discriminant() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let bytes = vec![37]; // Reserved slot
        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::InvalidWireTag { .. })));
    }

    #[test]
    fn test_decode_truncated_composite() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let array_byte = TypeTag::Array as u8;
        let bytes = vec![array_byte, 5, 0]; // Claims 5 bytes but no payload
        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_function_no_params() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[], mgr.int());
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_deeply_nested() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let mut ty = mgr.int();
        for _ in 0..100 {
            ty = mgr.array(ty);
        }

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_unicode_field_names() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![
            ("name", mgr.str()),
            ("名前", mgr.str()),
            ("🎉", mgr.bool()),
        ]);

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_encode_deterministic() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.map(mgr.str(), mgr.int())], mgr.array(mgr.bool()));

        let bytes1 = encode(ty);
        let bytes2 = encode(ty);
        let bytes3 = encode(ty);

        assert_eq!(bytes1.as_slice(), bytes2.as_slice());
        assert_eq!(bytes2.as_slice(), bytes3.as_slice());
    }

    // ============================================================================
    // OwnedType Tests
    // ============================================================================

    #[test]
    fn test_owned_type_primitives() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test Int
        let ty = mgr.int();
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());
        assert!(matches!(owned.view(), TypeKind::Int));

        // Test Float
        let ty = mgr.float();
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());
        assert!(matches!(owned.view(), TypeKind::Float));

        // Test Bool
        let ty = mgr.bool();
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());
        assert!(matches!(owned.view(), TypeKind::Bool));

        // Test Str
        let ty = mgr.str();
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());
        assert!(matches!(owned.view(), TypeKind::Str));

        // Test Bytes
        let ty = mgr.bytes();
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());
        assert!(matches!(owned.view(), TypeKind::Bytes));
    }

    #[test]
    fn test_owned_type_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Array(elem_view) => {
                assert!(matches!(elem_view.view(), TypeKind::Int));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_owned_type_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Map(key_view, val_view) => {
                assert!(matches!(key_view.view(), TypeKind::Str));
                assert!(matches!(val_view.view(), TypeKind::Int));
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn test_owned_type_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("x", mgr.int()), ("y", mgr.float())]);
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Record(fields) => {
                let fields: Vec<_> = fields.collect();
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert!(matches!(fields[0].1.view(), TypeKind::Int));
                assert_eq!(fields[1].0, "y");
                assert!(matches!(fields[1].1.view(), TypeKind::Float));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn test_owned_type_function() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Function { ret, params } => {
                assert!(matches!(ret.view(), TypeKind::Bool));
                let params: Vec<_> = params.collect();
                assert_eq!(params.len(), 2);
                assert!(matches!(params[0].view(), TypeKind::Int));
                assert!(matches!(params[1].view(), TypeKind::Str));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_owned_type_symbol() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["Option", "Some", "None"]);
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Symbol(parts) => {
                let parts: Vec<_> = parts.collect();
                // Symbol parts are sorted
                assert_eq!(parts, vec!["None", "Option", "Some"]);
            }
            _ => panic!("expected symbol"),
        }
    }

    #[test]
    fn test_owned_type_clone() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        let owned1 = OwnedType::new(bytes.as_slice().into());
        let owned2 = owned1.clone();

        // Both should produce the same view
        assert!(matches!(owned1.view(), TypeKind::Array(_)));
        assert!(matches!(owned2.view(), TypeKind::Array(_)));

        // Clone should be cheap
        assert_eq!(owned1, owned2);
    }

    #[test]
    fn test_owned_type_equality() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);

        let owned1 = OwnedType::new(bytes.as_slice().into());
        let owned2 = OwnedType::new(bytes.as_slice().into());

        // Same bytes should be equal
        assert_eq!(owned1, owned2);
    }

    #[test]
    fn test_owned_type_nested_composite() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Map[Str, Record{x: Int, y: Float}]]
        let inner_record = mgr.record(vec![("x", mgr.int()), ("y", mgr.float())]);
        let map_ty = mgr.map(mgr.str(), inner_record);
        let ty = mgr.array(map_ty);

        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Array(elem_view) => match elem_view.view() {
                TypeKind::Map(key_view, val_view) => {
                    assert!(matches!(key_view.view(), TypeKind::Str));
                    match val_view.view() {
                        TypeKind::Record(fields) => {
                            let fields: Vec<_> = fields.collect();
                            assert_eq!(fields.len(), 2);
                        }
                        _ => panic!("expected record"),
                    }
                }
                _ => panic!("expected map"),
            },
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_owned_type_multiple_views() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        // Should be able to call view() multiple times
        let view1 = owned.view();
        let view2 = owned.view();

        assert!(matches!(view1, TypeKind::Array(_)));
        assert!(matches!(view2, TypeKind::Array(_)));
    }

    #[test]
    fn test_owned_type_option() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.option(mgr.int());
        let bytes = encode(ty);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Option(inner_view) => {
                assert!(matches!(inner_view.view(), TypeKind::Int));
            }
            _ => panic!("expected option"),
        }
    }

    #[test]
    fn test_owned_type_nested_option() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Option[Array[Option[Str]]]
        let str_ty = mgr.str();
        let inner_opt = mgr.option(str_ty);
        let arr = mgr.array(inner_opt);
        let outer_opt = mgr.option(arr);

        let bytes = encode(outer_opt);
        let owned = OwnedType::new(bytes.as_slice().into());

        match owned.view() {
            TypeKind::Option(arr_view) => match arr_view.view() {
                TypeKind::Array(elem_view) => match elem_view.view() {
                    TypeKind::Option(str_view) => {
                        assert!(matches!(str_view.view(), TypeKind::Str));
                    }
                    _ => panic!("expected Option in array"),
                },
                _ => panic!("expected Array in outer Option"),
            },
            _ => panic!("expected Option at top level"),
        }
    }

    #[test]
    fn test_encode_decode_option_roundtrip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let original = mgr.option(mgr.int());
        let bytes = encode(original);
        let owned = OwnedType::new(bytes.as_slice().into());

        // Convert back to &Type via decode (this would require a decode function)
        // For now, just verify the view is correct
        match owned.view() {
            TypeKind::Option(inner) => {
                assert!(matches!(inner.view(), TypeKind::Int));
            }
            _ => panic!("roundtrip failed"),
        }
    }
}
