#[cfg(feature = "dtype-struct")]
use polars_utils::format_smartstring;
#[cfg(feature = "dtype-struct")]
use polars_utils::slice::GetSaferUnchecked;

use super::*;
#[cfg(feature = "dtype-struct")]
use crate::prelude::any_value::arr_to_any_value;

pub enum AnyValueBuffer<'a> {
    Boolean(BooleanChunkedBuilder),
    #[cfg(feature = "dtype-i8")]
    Int8(PrimitiveChunkedBuilder<Int8Type>),
    #[cfg(feature = "dtype-i16")]
    Int16(PrimitiveChunkedBuilder<Int16Type>),
    Int32(PrimitiveChunkedBuilder<Int32Type>),
    Int64(PrimitiveChunkedBuilder<Int64Type>),
    #[cfg(feature = "dtype-u8")]
    UInt8(PrimitiveChunkedBuilder<UInt8Type>),
    #[cfg(feature = "dtype-u16")]
    UInt16(PrimitiveChunkedBuilder<UInt16Type>),
    UInt32(PrimitiveChunkedBuilder<UInt32Type>),
    UInt64(PrimitiveChunkedBuilder<UInt64Type>),
    #[cfg(feature = "dtype-date")]
    Date(PrimitiveChunkedBuilder<Int32Type>),
    #[cfg(feature = "dtype-datetime")]
    Datetime(
        PrimitiveChunkedBuilder<Int64Type>,
        TimeUnit,
        Option<TimeZone>,
    ),
    #[cfg(feature = "dtype-duration")]
    Duration(PrimitiveChunkedBuilder<Int64Type>, TimeUnit),
    #[cfg(feature = "dtype-time")]
    Time(PrimitiveChunkedBuilder<Int64Type>),
    Float32(PrimitiveChunkedBuilder<Float32Type>),
    Float64(PrimitiveChunkedBuilder<Float64Type>),
    Utf8(Utf8ChunkedBuilder),
    All(DataType, Vec<AnyValue<'a>>),
}

impl<'a> AnyValueBuffer<'a> {
    #[inline]
    pub fn add(&mut self, val: AnyValue<'a>) -> Option<()> {
        use AnyValueBuffer::*;
        match (self, val) {
            (Boolean(builder), AnyValue::Null) => builder.append_null(),
            (Boolean(builder), AnyValue::Boolean(v)) => builder.append_value(v),
            (Boolean(builder), val) => {
                let v = val.extract::<u8>()?;
                builder.append_value(v == 1)
            }
            (Int32(builder), AnyValue::Null) => builder.append_null(),
            (Int32(builder), val) => builder.append_value(val.extract()?),
            (Int64(builder), AnyValue::Null) => builder.append_null(),
            (Int64(builder), val) => builder.append_value(val.extract()?),
            #[cfg(feature = "dtype-u8")]
            (UInt8(builder), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-u16")]
            (UInt16(builder), val) => builder.append_value(val.extract()?),
            (UInt32(builder), AnyValue::Null) => builder.append_null(),
            (UInt32(builder), val) => builder.append_value(val.extract()?),
            (UInt64(builder), AnyValue::Null) => builder.append_null(),
            (UInt64(builder), val) => builder.append_value(val.extract()?),
            (Float32(builder), AnyValue::Null) => builder.append_null(),
            (Float64(builder), AnyValue::Null) => builder.append_null(),
            (Float32(builder), val) => builder.append_value(val.extract()?),
            (Float64(builder), val) => builder.append_value(val.extract()?),
            (Utf8(builder), AnyValue::Utf8(v)) => builder.append_value(v),
            (Utf8(builder), AnyValue::Utf8Owned(v)) => builder.append_value(v),
            (Utf8(builder), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-i8")]
            (Int8(builder), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-i8")]
            (Int8(builder), val) => builder.append_value(val.extract()?),
            #[cfg(feature = "dtype-i16")]
            (Int16(builder), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-i16")]
            (Int16(builder), val) => builder.append_value(val.extract()?),
            #[cfg(feature = "dtype-date")]
            (Date(builder), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-date")]
            (Date(builder), AnyValue::Date(v)) => builder.append_value(v),
            #[cfg(feature = "dtype-datetime")]
            (Datetime(builder, _, _), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-datetime")]
            (Datetime(builder, tu_l, _), AnyValue::Datetime(v, tu_r, _)) => {
                // we convert right tu to left tu
                // so we swap.
                let v = convert_time_units(v, tu_r, *tu_l);
                builder.append_value(v)
            }
            #[cfg(feature = "dtype-duration")]
            (Duration(builder, _), AnyValue::Null) => builder.append_null(),
            #[cfg(feature = "dtype-duration")]
            (Duration(builder, tu_l), AnyValue::Duration(v, tu_r)) => {
                let v = convert_time_units(v, tu_r, *tu_l);
                builder.append_value(v)
            }
            #[cfg(feature = "dtype-time")]
            (Time(builder), AnyValue::Time(v)) => builder.append_value(v),
            #[cfg(feature = "dtype-time")]
            (Time(builder), AnyValue::Null) => builder.append_null(),
            // Struct and List can be recursive so use anyvalues for that
            (All(_, vals), v) => vals.push(v),

            // dynamic types
            (Utf8(builder), av) => match av {
                AnyValue::Int64(v) => builder.append_value(&format!("{v}")),
                AnyValue::Float64(v) => builder.append_value(&format!("{v}")),
                AnyValue::Boolean(true) => builder.append_value("true"),
                AnyValue::Boolean(false) => builder.append_value("false"),
                _ => return None,
            },
            _ => return None,
        };
        Some(())
    }

    pub(crate) fn add_fallible(&mut self, val: &AnyValue<'a>) -> PolarsResult<()> {
        self.add(val.clone()).ok_or_else(|| {
            polars_err!(
                ComputeError: "could not append {:?} to the builder; make sure that all rows \
                have the same schema or consider increasing `schema_inference_length`"
            )
        })
    }

    pub fn into_series(self) -> Series {
        use AnyValueBuffer::*;
        match self {
            Boolean(b) => b.finish().into_series(),
            Int32(b) => b.finish().into_series(),
            Int64(b) => b.finish().into_series(),
            UInt32(b) => b.finish().into_series(),
            UInt64(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-date")]
            Date(b) => b.finish().into_date().into_series(),
            #[cfg(feature = "dtype-datetime")]
            Datetime(b, tu, tz) => b.finish().into_datetime(tu, tz).into_series(),
            #[cfg(feature = "dtype-duration")]
            Duration(b, tu) => b.finish().into_duration(tu).into_series(),
            #[cfg(feature = "dtype-time")]
            Time(b) => b.finish().into_time().into_series(),
            Float32(b) => b.finish().into_series(),
            Float64(b) => b.finish().into_series(),
            Utf8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-i8")]
            Int8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-i16")]
            Int16(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-u8")]
            UInt8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-u16")]
            UInt16(b) => b.finish().into_series(),
            All(dtype, vals) => {
                Series::from_any_values_and_dtype("", &vals, &dtype, false).unwrap()
            }
        }
    }

    pub fn new(dtype: &DataType, capacity: usize) -> AnyValueBuffer<'a> {
        (dtype, capacity).into()
    }
}

// datatype and length
impl From<(&DataType, usize)> for AnyValueBuffer<'_> {
    fn from(a: (&DataType, usize)) -> Self {
        let (dt, len) = a;
        use DataType::*;
        match dt {
            Boolean => AnyValueBuffer::Boolean(BooleanChunkedBuilder::new("", len)),
            Int32 => AnyValueBuffer::Int32(PrimitiveChunkedBuilder::new("", len)),
            Int64 => AnyValueBuffer::Int64(PrimitiveChunkedBuilder::new("", len)),
            UInt32 => AnyValueBuffer::UInt32(PrimitiveChunkedBuilder::new("", len)),
            UInt64 => AnyValueBuffer::UInt64(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-i8")]
            Int8 => AnyValueBuffer::Int8(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-i16")]
            Int16 => AnyValueBuffer::Int16(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-u8")]
            UInt8 => AnyValueBuffer::UInt8(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-u16")]
            UInt16 => AnyValueBuffer::UInt16(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-date")]
            Date => AnyValueBuffer::Date(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-datetime")]
            Datetime(tu, tz) => {
                AnyValueBuffer::Datetime(PrimitiveChunkedBuilder::new("", len), *tu, tz.clone())
            }
            #[cfg(feature = "dtype-duration")]
            Duration(tu) => AnyValueBuffer::Duration(PrimitiveChunkedBuilder::new("", len), *tu),
            #[cfg(feature = "dtype-time")]
            Time => AnyValueBuffer::Time(PrimitiveChunkedBuilder::new("", len)),
            Float32 => AnyValueBuffer::Float32(PrimitiveChunkedBuilder::new("", len)),
            Float64 => AnyValueBuffer::Float64(PrimitiveChunkedBuilder::new("", len)),
            Utf8 => AnyValueBuffer::Utf8(Utf8ChunkedBuilder::new("", len, len * 5)),
            // Struct and List can be recursive so use anyvalues for that
            dt => AnyValueBuffer::All(dt.clone(), Vec::with_capacity(len)),
        }
    }
}

//// An `AnyValyeBuffer` that should be used when we trust the builder
pub enum AnyValueBufferTrusted<'a> {
    Boolean(BooleanChunkedBuilder),
    #[cfg(feature = "dtype-i8")]
    Int8(PrimitiveChunkedBuilder<Int8Type>),
    #[cfg(feature = "dtype-i16")]
    Int16(PrimitiveChunkedBuilder<Int16Type>),
    Int32(PrimitiveChunkedBuilder<Int32Type>),
    Int64(PrimitiveChunkedBuilder<Int64Type>),
    #[cfg(feature = "dtype-u8")]
    UInt8(PrimitiveChunkedBuilder<UInt8Type>),
    #[cfg(feature = "dtype-u16")]
    UInt16(PrimitiveChunkedBuilder<UInt16Type>),
    UInt32(PrimitiveChunkedBuilder<UInt32Type>),
    UInt64(PrimitiveChunkedBuilder<UInt64Type>),
    Float32(PrimitiveChunkedBuilder<Float32Type>),
    Float64(PrimitiveChunkedBuilder<Float64Type>),
    Utf8(Utf8ChunkedBuilder),
    #[cfg(feature = "dtype-struct")]
    // not the trusted variant!
    Struct(Vec<AnyValueBuffer<'a>>),
    All(DataType, Vec<AnyValue<'a>>),
}

impl<'a> AnyValueBufferTrusted<'a> {
    /// Will add the AnyValue into `Self` and unpack as the physical type
    /// belonging to `Self`. This should only be used with physical buffers
    ///
    /// If a type is not primitive or utf8, the anyvalue will be converted to static
    ///
    /// # Safety
    /// The caller must ensure that the `AnyValue` type exactly matches the `Buffer` type.
    pub unsafe fn add_unchecked_owned_physical(&mut self, val: &AnyValue<'_>) {
        use AnyValueBufferTrusted::*;
        match val {
            AnyValue::Null => match self {
                Boolean(builder) => builder.append_null(),
                #[cfg(feature = "dtype-i8")]
                Int8(builder) => builder.append_null(),
                #[cfg(feature = "dtype-i16")]
                Int16(builder) => builder.append_null(),
                Int32(builder) => builder.append_null(),
                Int64(builder) => builder.append_null(),
                #[cfg(feature = "dtype-u8")]
                UInt8(builder) => builder.append_null(),
                #[cfg(feature = "dtype-u16")]
                UInt16(builder) => builder.append_null(),
                UInt32(builder) => builder.append_null(),
                UInt64(builder) => builder.append_null(),
                Float32(builder) => builder.append_null(),
                Float64(builder) => builder.append_null(),
                Utf8(builder) => builder.append_null(),
                #[cfg(feature = "dtype-struct")]
                Struct(builders) => {
                    for b in builders.iter_mut() {
                        b.add(AnyValue::Null);
                    }
                }
                All(_, vals) => vals.push(val.clone().into_static().unwrap()),
            },
            _ => {
                match self {
                    Boolean(builder) => {
                        let AnyValue::Boolean(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    #[cfg(feature = "dtype-i8")]
                    Int8(builder) => {
                        let AnyValue::Int8(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    #[cfg(feature = "dtype-i16")]
                    Int16(builder) => {
                        let AnyValue::Int16(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    Int32(builder) => {
                        let AnyValue::Int32(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    Int64(builder) => {
                        let AnyValue::Int64(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    #[cfg(feature = "dtype-u8")]
                    UInt8(builder) => {
                        let AnyValue::UInt8(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    #[cfg(feature = "dtype-u16")]
                    UInt16(builder) => {
                        let AnyValue::UInt16(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    UInt32(builder) => {
                        let AnyValue::UInt32(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    UInt64(builder) => {
                        let AnyValue::UInt64(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    Float32(builder) => {
                        let AnyValue::Float32(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    Float64(builder) => {
                        let AnyValue::Float64(v) = val else { unreachable_unchecked() };
                        builder.append_value(*v)
                    }
                    Utf8(builder) => {
                        let AnyValue::Utf8(v) = val else { unreachable_unchecked() };
                        builder.append_value(v)
                    }
                    #[cfg(feature = "dtype-struct")]
                    Struct(builders) => {
                        let AnyValue::Struct(idx, arr, fields) = val else { unreachable_unchecked() };
                        let arrays = arr.values();
                        // amortize loop counter
                        for i in 0..fields.len() {
                            unsafe {
                                let array = arrays.get_unchecked_release(i);
                                let field = fields.get_unchecked_release(i);
                                let builder = builders.get_unchecked_release_mut(i);
                                let av = arr_to_any_value(&**array, *idx, &field.dtype);
                                // lifetime is bound to 'a
                                let av = std::mem::transmute::<AnyValue<'_>, AnyValue<'a>>(av);
                                builder.add(av);
                            }
                        }
                    }
                    All(_, vals) => vals.push(val.clone().into_static().unwrap()),
                }
            }
        }
    }

    pub fn into_series(self) -> Series {
        use AnyValueBufferTrusted::*;
        match self {
            Boolean(b) => b.finish().into_series(),
            Int32(b) => b.finish().into_series(),
            Int64(b) => b.finish().into_series(),
            UInt32(b) => b.finish().into_series(),
            UInt64(b) => b.finish().into_series(),
            Float32(b) => b.finish().into_series(),
            Float64(b) => b.finish().into_series(),
            Utf8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-i8")]
            Int8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-i16")]
            Int16(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-u8")]
            UInt8(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-u16")]
            UInt16(b) => b.finish().into_series(),
            #[cfg(feature = "dtype-struct")]
            Struct(b) => {
                let v = b
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| {
                        let mut s = b.into_series();
                        s.rename(format_smartstring!("field_{}", i).as_str());
                        s
                    })
                    .collect::<Vec<_>>();
                StructChunked::new("", &v).unwrap().into_series()
            }
            All(dtype, vals) => {
                Series::from_any_values_and_dtype("", &vals, &dtype, false).unwrap()
            }
        }
    }
}
impl From<(&DataType, usize)> for AnyValueBufferTrusted<'_> {
    fn from(a: (&DataType, usize)) -> Self {
        let (dt, len) = a;
        use DataType::*;
        match dt {
            Boolean => AnyValueBufferTrusted::Boolean(BooleanChunkedBuilder::new("", len)),
            Int32 => AnyValueBufferTrusted::Int32(PrimitiveChunkedBuilder::new("", len)),
            Int64 => AnyValueBufferTrusted::Int64(PrimitiveChunkedBuilder::new("", len)),
            UInt32 => AnyValueBufferTrusted::UInt32(PrimitiveChunkedBuilder::new("", len)),
            UInt64 => AnyValueBufferTrusted::UInt64(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-i8")]
            Int8 => AnyValueBufferTrusted::Int8(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-i16")]
            Int16 => AnyValueBufferTrusted::Int16(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-u8")]
            UInt8 => AnyValueBufferTrusted::UInt8(PrimitiveChunkedBuilder::new("", len)),
            #[cfg(feature = "dtype-u16")]
            UInt16 => AnyValueBufferTrusted::UInt16(PrimitiveChunkedBuilder::new("", len)),
            Float32 => AnyValueBufferTrusted::Float32(PrimitiveChunkedBuilder::new("", len)),
            Float64 => AnyValueBufferTrusted::Float64(PrimitiveChunkedBuilder::new("", len)),
            Utf8 => AnyValueBufferTrusted::Utf8(Utf8ChunkedBuilder::new("", len, len * 5)),
            #[cfg(feature = "dtype-struct")]
            Struct(fields) => {
                let buffers = fields
                    .iter()
                    .map(|field| {
                        let dtype = field.data_type().to_physical();
                        let buffer: AnyValueBuffer = (&dtype, len).into();
                        buffer
                    })
                    .collect::<Vec<_>>();
                AnyValueBufferTrusted::Struct(buffers)
            }
            // List can be recursive so use anyvalues for that
            dt => AnyValueBufferTrusted::All(dt.clone(), Vec::with_capacity(len)),
        }
    }
}