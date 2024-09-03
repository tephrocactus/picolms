use ipnet::IpNet;
use like::ILike;
use ordered_float::OrderedFloat;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::net::IpAddr;
use time::OffsetDateTime;
use unicase::UniCase;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Value<'a> {
    String(Cow<'a, str>),
    VecString(Cow<'a, [String]>),
    I64(i64),
    VecI64(Cow<'a, [i64]>),
    F64(OrderedFloat<f64>),
    VecF64(Cow<'a, [OrderedFloat<f64>]>),
    Timestamp(OffsetDateTime),
    VecTimestamp(Cow<'a, [OffsetDateTime]>),
    Ip(IpAddr),
    VecIp(Cow<'a, [IpAddr]>),
    IpNet(IpNet),
    VecIpNet(Cow<'a, [IpNet]>),
    Uuid(Uuid),
    VecUuid(Cow<'a, [Uuid]>),
    Bool(bool),
    VecBool(Cow<'a, [bool]>),
    Null,
}

impl<'a> Value<'a> {
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::String(v) => Value::String(Cow::Owned(v.into_owned())),
            Value::VecString(v) => Value::VecString(Cow::Owned(v.into_owned())),

            Value::I64(v) => Value::I64(v),
            Value::VecI64(v) => Value::VecI64(Cow::Owned(v.into_owned())),

            Value::F64(v) => Value::F64(v),
            Value::VecF64(v) => Value::VecF64(Cow::Owned(v.into_owned())),

            Value::Timestamp(v) => Value::Timestamp(v),
            Value::VecTimestamp(v) => Value::VecTimestamp(Cow::Owned(v.into_owned())),

            Value::Ip(v) => Value::Ip(v),
            Value::VecIp(v) => Value::VecIp(Cow::Owned(v.into_owned())),

            Value::IpNet(v) => Value::IpNet(v),
            Value::VecIpNet(v) => Value::VecIpNet(Cow::Owned(v.into_owned())),

            Value::Uuid(v) => Value::Uuid(v),
            Value::VecUuid(v) => Value::VecUuid(Cow::Owned(v.into_owned())),

            Value::Bool(v) => Value::Bool(v),
            Value::VecBool(v) => Value::VecBool(Cow::Owned(v.into_owned())),

            Value::Null => Value::Null,
        }
    }

    pub fn to_borrowed(&'a self) -> Value<'a> {
        match self {
            Value::String(v) => Value::String(Cow::Borrowed(v)),
            Value::VecString(v) => Value::VecString(Cow::Borrowed(v)),

            Value::I64(v) => Value::I64(*v),
            Value::VecI64(v) => Value::VecI64(Cow::Borrowed(v)),

            Value::F64(v) => Value::F64(*v),
            Value::VecF64(v) => Value::VecF64(Cow::Borrowed(v)),

            Value::Timestamp(v) => Value::Timestamp(*v),
            Value::VecTimestamp(v) => Value::VecTimestamp(Cow::Borrowed(v)),

            Value::Ip(v) => Value::Ip(*v),
            Value::VecIp(v) => Value::VecIp(Cow::Borrowed(v)),

            Value::IpNet(v) => Value::IpNet(*v),
            Value::VecIpNet(v) => Value::VecIpNet(Cow::Borrowed(v)),

            Value::Uuid(v) => Value::Uuid(*v),
            Value::VecUuid(v) => Value::VecUuid(Cow::Borrowed(v)),

            Value::Bool(v) => Value::Bool(*v),
            Value::VecBool(v) => Value::VecBool(Cow::Borrowed(v)),

            Value::Null => Value::Null,
        }
    }

    pub fn eq_fold(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => UniCase::unicode(lv) == UniCase::unicode(rv),
            (Self::VecString(lv), Self::VecString(rv)) if lv.len() == rv.len() => lv
                .iter()
                .zip(rv.iter())
                .all(|(lv, rv)| UniCase::unicode(lv) == UniCase::unicode(rv)),
            _ => false,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => lv.contains(rv.as_ref()),
            (Self::VecString(lv), Self::String(rv)) => lv.iter().any(|lv| lv == rv),
            (Self::VecI64(lv), Self::I64(rv)) => lv.contains(rv),
            (Self::VecF64(lv), Self::F64(rv)) => lv.contains(rv),
            (Self::VecTimestamp(lv), Self::Timestamp(rv)) => lv.contains(rv),
            (Self::VecIp(lv), Self::Ip(rv)) => lv.contains(rv),
            (Self::IpNet(lv), Self::Ip(rv)) => lv.contains(rv),
            (Self::VecIpNet(lv), Self::Ip(rv)) => lv.iter().any(|lv| lv.contains(rv)),
            (Self::VecIpNet(lv), Self::IpNet(rv)) => lv.contains(rv),
            (Self::VecUuid(lv), Self::Uuid(rv)) => lv.contains(rv),
            (Self::VecBool(lv), Self::Bool(rv)) => lv.contains(rv),
            _ => false,
        }
    }

    pub fn contains_fold(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => {
                ILike::<false>::ilike(lv.as_ref(), &format!("%{}%", rv)).unwrap_or_default()
            }
            (Self::VecString(lv), Self::String(rv)) => {
                let rv = UniCase::unicode(rv);
                lv.iter().any(|lv| UniCase::unicode(lv) == rv)
            }
            _ => false,
        }
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => lv.starts_with(rv.as_ref()),
            (Self::VecString(lv), Self::String(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecString(lv), Self::VecString(rv)) => lv.starts_with(rv),

            (Self::VecI64(lv), Self::I64(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecI64(lv), Self::VecI64(rv)) => lv.starts_with(rv),

            (Self::VecF64(lv), Self::F64(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecF64(lv), Self::VecF64(rv)) => lv.starts_with(rv),

            (Self::VecTimestamp(lv), Self::Timestamp(rv)) => {
                lv.first().map_or(false, |lv| lv == rv)
            }
            (Self::VecTimestamp(lv), Self::VecTimestamp(rv)) => lv.starts_with(rv),

            (Self::VecIp(lv), Self::Ip(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecIp(lv), Self::VecIp(rv)) => lv.starts_with(rv),

            (Self::VecUuid(lv), Self::Uuid(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecUuid(lv), Self::VecUuid(rv)) => lv.starts_with(rv),

            (Self::VecBool(lv), Self::Bool(rv)) => lv.first().map_or(false, |lv| lv == rv),
            (Self::VecBool(lv), Self::VecBool(rv)) => lv.starts_with(rv),

            _ => false,
        }
    }

    pub fn starts_with_fold(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => {
                ILike::<false>::ilike(lv.as_ref(), &format!("{}%", rv)).unwrap_or_default()
            }
            (Self::VecString(lv), Self::String(rv)) => lv
                .first()
                .map_or(false, |lv| UniCase::unicode(lv) == UniCase::unicode(rv)),
            (Self::VecString(lv), Self::VecString(rv)) if lv.len() >= rv.len() => lv
                .iter()
                .zip(rv.iter())
                .all(|(lv, rv)| UniCase::unicode(lv) == UniCase::unicode(rv)),
            _ => false,
        }
    }

    pub fn ends_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => lv.ends_with(rv.as_ref()),
            (Self::VecString(lv), Self::String(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecString(lv), Self::VecString(rv)) => lv.ends_with(rv),

            (Self::VecI64(lv), Self::I64(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecI64(lv), Self::VecI64(rv)) => lv.ends_with(rv),

            (Self::VecF64(lv), Self::F64(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecF64(lv), Self::VecF64(rv)) => lv.ends_with(rv),

            (Self::VecTimestamp(lv), Self::Timestamp(rv)) => {
                lv.first().map_or(false, |lv| lv == rv)
            }
            (Self::VecTimestamp(lv), Self::VecTimestamp(rv)) => lv.ends_with(rv),

            (Self::VecIp(lv), Self::Ip(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecIp(lv), Self::VecIp(rv)) => lv.ends_with(rv),

            (Self::VecUuid(lv), Self::Uuid(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecUuid(lv), Self::VecUuid(rv)) => lv.ends_with(rv),

            (Self::VecBool(lv), Self::Bool(rv)) => lv.last().map_or(false, |lv| lv == rv),
            (Self::VecBool(lv), Self::VecBool(rv)) => lv.ends_with(rv),

            _ => false,
        }
    }

    pub fn ends_with_fold(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(lv), Self::String(rv)) => {
                ILike::<false>::ilike(lv.as_ref(), &format!("%{}", rv)).unwrap_or_default()
            }
            (Self::VecString(lv), Self::String(rv)) => lv
                .last()
                .map_or(false, |lv| UniCase::unicode(lv) == UniCase::unicode(rv)),
            (Self::VecString(lv), Self::VecString(rv)) if lv.len() >= rv.len() => lv
                .iter()
                .rev()
                .zip(rv.iter().rev())
                .all(|(lv, rv)| UniCase::unicode(lv) == UniCase::unicode(rv)),
            _ => false,
        }
    }

    pub fn matches(&self, regex: &Regex) -> bool {
        match self {
            Self::String(v) => regex.is_match(v),
            _ => false,
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::I64(lv), Self::I64(rv)) => lv.saturating_add(*rv).into(),
            (Self::F64(lv), Self::F64(rv)) => Value::F64(*lv + *rv),
            _ => Value::Null,
        }
    }

    pub fn sub(&self, other: &Self) -> Value {
        match (self, other) {
            (Self::I64(lv), Self::I64(rv)) => lv.saturating_sub(*rv).into(),
            (Self::F64(lv), Self::F64(rv)) => Value::F64(*lv - *rv),
            _ => Value::Null,
        }
    }

    pub fn mul(&self, other: &Self) -> Value {
        match (self, other) {
            (Self::I64(lv), Self::I64(rv)) => lv.saturating_mul(*rv).into(),
            (Self::F64(lv), Self::F64(rv)) => Value::F64(*lv * *rv),
            _ => Value::Null,
        }
    }

    pub fn div(&self, other: &Self) -> Value {
        match (self, other) {
            (Self::I64(lv), Self::I64(rv)) if *rv != 0 => lv.saturating_div(*rv).into(),
            (Self::F64(lv), Self::F64(rv)) if *rv != 0.0 => Value::F64(*lv / *rv),
            _ => Value::Null,
        }
    }

    pub fn trim(&self) -> Self {
        match self {
            Self::String(v) => v.trim().to_string().into(),
            _ => Self::Null,
        }
    }

    pub fn to_lowercase(&self) -> Self {
        match self {
            Self::String(v) => v.clone().to_lowercase().into(),
            _ => Self::Null,
        }
    }

    pub fn to_uppercase(&self) -> Self {
        match self {
            Self::String(v) => v.clone().to_uppercase().into(),
            _ => Self::Null,
        }
    }
}

//
// T -> Value.
//

impl From<String> for Value<'static> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

impl From<Vec<String>> for Value<'static> {
    fn from(value: Vec<String>) -> Self {
        Self::VecString(Cow::Owned(value))
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'a> From<&'a [String]> for Value<'a> {
    fn from(value: &'a [String]) -> Self {
        Self::VecString(Cow::Borrowed(value))
    }
}

impl From<i64> for Value<'static> {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<Vec<i64>> for Value<'static> {
    fn from(value: Vec<i64>) -> Self {
        Self::VecI64(Cow::Owned(value))
    }
}

impl<'a> From<&'a [i64]> for Value<'a> {
    fn from(value: &'a [i64]) -> Self {
        Self::VecI64(Cow::Borrowed(value))
    }
}

impl From<f64> for Value<'static> {
    fn from(value: f64) -> Self {
        Self::F64(value.into())
    }
}

impl From<OrderedFloat<f64>> for Value<'static> {
    fn from(value: OrderedFloat<f64>) -> Self {
        Self::F64(value)
    }
}

impl From<Vec<OrderedFloat<f64>>> for Value<'static> {
    fn from(value: Vec<OrderedFloat<f64>>) -> Self {
        Self::VecF64(Cow::Owned(value))
    }
}

impl<'a> From<&'a [OrderedFloat<f64>]> for Value<'a> {
    fn from(value: &'a [OrderedFloat<f64>]) -> Self {
        Self::VecF64(Cow::Borrowed(value))
    }
}

impl From<OffsetDateTime> for Value<'static> {
    fn from(value: OffsetDateTime) -> Self {
        Self::Timestamp(value)
    }
}

impl From<Vec<OffsetDateTime>> for Value<'static> {
    fn from(value: Vec<OffsetDateTime>) -> Self {
        Self::VecTimestamp(Cow::Owned(value))
    }
}

impl<'a> From<&'a [OffsetDateTime]> for Value<'a> {
    fn from(value: &'a [OffsetDateTime]) -> Self {
        Self::VecTimestamp(Cow::Borrowed(value))
    }
}

impl From<IpAddr> for Value<'static> {
    fn from(value: IpAddr) -> Self {
        Self::Ip(value)
    }
}

impl From<Vec<IpAddr>> for Value<'static> {
    fn from(value: Vec<IpAddr>) -> Self {
        Self::VecIp(Cow::Owned(value))
    }
}

impl<'a> From<&'a [IpAddr]> for Value<'a> {
    fn from(value: &'a [IpAddr]) -> Self {
        Self::VecIp(Cow::Borrowed(value))
    }
}

impl From<bool> for Value<'static> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Vec<bool>> for Value<'static> {
    fn from(value: Vec<bool>) -> Self {
        Self::VecBool(Cow::Owned(value))
    }
}

impl<'a> From<&'a [bool]> for Value<'a> {
    fn from(value: &'a [bool]) -> Self {
        Self::VecBool(Cow::Borrowed(value))
    }
}

impl From<Uuid> for Value<'static> {
    fn from(value: Uuid) -> Self {
        Self::Uuid(value)
    }
}

impl From<Vec<Uuid>> for Value<'static> {
    fn from(value: Vec<Uuid>) -> Self {
        Self::VecUuid(Cow::Owned(value))
    }
}

impl<'a> From<&'a [Uuid]> for Value<'a> {
    fn from(value: &'a [Uuid]) -> Self {
        Self::VecUuid(Cow::Borrowed(value))
    }
}

impl From<Vec<IpNet>> for Value<'static> {
    fn from(value: Vec<IpNet>) -> Self {
        Self::VecIpNet(Cow::Owned(value))
    }
}

impl<'a> From<&'a [IpNet]> for Value<'a> {
    fn from(value: &'a [IpNet]) -> Self {
        Self::VecIpNet(Cow::Borrowed(value))
    }
}
